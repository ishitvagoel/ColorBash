use crate::error::PtyError;
use crate::sys::{self, Termios};
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

pub const CTRL_C: u8 = 0x03;
pub const CTRL_Z: u8 = 0x1A;
pub const CTRL_D: u8 = 0x04;
pub const SIGKILL: i32 = 9;
pub const SIGTERM: i32 = 15;
pub const DEFAULT_CAPTURE_LIMIT: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WinSize {
    pub rows: u16,
    pub cols: u16,
}

impl Default for WinSize {
    fn default() -> Self {
        Self { rows: 24, cols: 80 }
    }
}

#[derive(Clone, Debug)]
pub struct SpawnOptions {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub env: Vec<(OsString, OsString)>,
    pub clear_env: bool,
    pub cwd: Option<PathBuf>,
    pub winsize: WinSize,
}

impl SpawnOptions {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            env: Vec::new(),
            clear_env: false,
            cwd: None,
            winsize: WinSize::default(),
        }
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    pub fn clear_env(mut self) -> Self {
        self.clear_env = true;
        self
    }

    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn winsize(mut self, winsize: WinSize) -> Self {
        self.winsize = winsize;
        self
    }
}

pub struct PtySession {
    master: File,
    child: Child,
}

impl PtySession {
    pub fn spawn(options: SpawnOptions) -> Result<Self, PtyError> {
        let master = sys::open_master()?;
        let slave_path = sys::slave_path(&master)?;
        let slave = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(sys::O_NOCTTY)
            .open(&slave_path)
            .map_err(PtyError::Open)?;
        let stdin = slave.try_clone().map_err(PtyError::Open)?;
        let stdout = slave.try_clone().map_err(PtyError::Open)?;
        let stderr = slave.try_clone().map_err(PtyError::Open)?;
        let winsize = options.winsize;
        sys::set_winsize(slave.as_raw_fd(), winsize).map_err(PtyError::Open)?;

        let mut command = Command::new(&options.program);
        command
            .args(&options.args)
            .stdin(Stdio::from(stdin))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        if options.clear_env {
            command.env_clear();
        }
        for (key, value) in &options.env {
            command.env(key, value);
        }
        if let Some(cwd) = &options.cwd {
            command.current_dir(cwd);
        }
        unsafe {
            command.pre_exec(move || {
                sys::become_session_leader()?;
                sys::set_controlling_tty(0)?;
                sys::set_winsize(0, winsize)?;
                Ok(())
            });
        }
        let child = command.spawn().map_err(PtyError::Spawn)?;
        drop(slave);
        let master = unsafe { File::from_raw_fd(master.into_raw_fd()) };
        Ok(Self { master, child })
    }

    pub fn child_pid(&self) -> u32 {
        self.child.id()
    }

    pub fn write_all(&mut self, bytes: &[u8], deadline: Instant) -> Result<(), PtyError> {
        let mut written = 0;
        while written < bytes.len() {
            match sys::wait_ready(self.master.as_raw_fd(), true, deadline) {
                Err(PtyError::Timeout(_)) => {
                    return Err(PtyError::Timeout(bytes[written..].to_vec()));
                }
                Err(error) => return Err(error),
                Ok(_) => {}
            }
            match self.master.write(&bytes[written..]) {
                Ok(0) => return Err(PtyError::ChildExited),
                Ok(amount) => written += amount,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => continue,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(PtyError::Io(error)),
            }
        }
        Ok(())
    }

    pub fn write_str(&mut self, text: &str, deadline: Instant) -> Result<(), PtyError> {
        self.write_all(text.as_bytes(), deadline)
    }

    pub fn read_until(
        &mut self,
        deadline: Instant,
        max_bytes: usize,
        mut predicate: impl FnMut(&[u8]) -> bool,
    ) -> Result<Vec<u8>, PtyError> {
        let mut captured = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            if predicate(&captured) {
                return Ok(captured);
            }
            if captured.len() >= max_bytes {
                return Err(PtyError::Oversize);
            }
            match sys::wait_ready(self.master.as_raw_fd(), false, deadline) {
                Err(PtyError::Timeout(_)) => return Err(PtyError::Timeout(captured)),
                Err(error) => return Err(error),
                Ok(_) => {}
            }
            match self.master.read(&mut chunk) {
                Ok(0) => {
                    if predicate(&captured) {
                        return Ok(captured);
                    }
                    return Err(PtyError::ChildExited);
                }
                Ok(amount) => {
                    let allowed = max_bytes.saturating_sub(captured.len());
                    if amount > allowed {
                        captured.extend_from_slice(&chunk[..allowed]);
                        return Err(PtyError::Oversize);
                    }
                    captured.extend_from_slice(&chunk[..amount]);
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => continue,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.raw_os_error() == Some(5) => {
                    if predicate(&captured) {
                        return Ok(captured);
                    }
                    return Err(PtyError::ChildExited);
                }
                Err(error) => return Err(PtyError::Io(error)),
            }
        }
    }

    pub fn resize(&mut self, size: WinSize) -> Result<(), PtyError> {
        sys::set_winsize(self.master.as_raw_fd(), size).map_err(PtyError::Io)
    }

    pub fn winsize(&self) -> Result<WinSize, PtyError> {
        sys::get_winsize(self.master.as_raw_fd()).map_err(PtyError::Io)
    }

    pub fn termios(&self) -> Result<Termios, PtyError> {
        sys::get_termios(self.master.as_raw_fd()).map_err(PtyError::Io)
    }

    pub fn kill(&mut self, signal: i32) -> Result<(), PtyError> {
        sys::send_signal(self.child_pid(), signal).map_err(PtyError::Io)
    }

    pub fn kill_group(&mut self, signal: i32) -> Result<(), PtyError> {
        sys::send_group_signal(self.child_pid(), signal).map_err(PtyError::Io)
    }

    pub fn wait(&mut self) -> Result<std::process::ExitStatus, PtyError> {
        self.child.wait().map_err(PtyError::Io)
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        let pid = self.child.id();
        let _ = sys::send_group_signal(pid, SIGKILL);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn contains(haystack: &[u8], needle: impl AsRef<[u8]>) -> bool {
    let needle = needle.as_ref();
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

pub fn contains_str(haystack: &[u8], needle: &str) -> bool {
    contains(haystack, needle.as_bytes())
}

pub fn visible_text(bytes: &[u8]) -> String {
    let mut visible = String::new();
    let lossy = String::from_utf8_lossy(bytes);
    let mut chars = lossy.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' {
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    for next in chars.by_ref() {
                        if ('\u{40}'..='\u{7e}').contains(&next) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    while let Some(next) = chars.next() {
                        if next == '\u{7}' {
                            break;
                        }
                        if next == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
                            break;
                        }
                    }
                }
                Some(_) => {
                    chars.next();
                }
                None => {}
            }
            continue;
        }
        if character != '\r' {
            visible.push(character);
        }
    }
    visible
}

pub fn visible_contains(haystack: &[u8], needle: &str) -> bool {
    visible_text(haystack).contains(needle)
}

#[cfg(test)]
mod tests {
    use super::{visible_contains, visible_text};

    #[test]
    fn visible_text_strips_cr_and_csi() {
        let raw = b"\x1b[?2004h~\r\r\n> ";
        assert_eq!(visible_text(raw), "~\n> ");
        assert!(visible_contains(raw, "\n> "));
    }

    #[test]
    fn visible_text_strips_csi_with_non_alpha_final_byte() {
        let raw = b"\x1b[1@ok\r\n";
        assert_eq!(visible_text(raw), "ok\n");
    }

    #[test]
    fn visible_text_strips_osc_with_st_terminator() {
        let raw = b"\x1b]0;title\x1b\\ok\r\n";
        assert_eq!(visible_text(raw), "ok\n");
    }
}
