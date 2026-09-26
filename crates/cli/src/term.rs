//! Minimal Linux terminal control for the modal TUI (ADR 0016): raw mode,
//! window size, polled input, and a signal waker. Hand-rolled FFI in the
//! house style of `crates/pty/src/sys.rs`; the workspace gains no new
//! dependencies. Other platforms compile and report a clean error instead.

#[cfg(target_os = "linux")]
mod imp {
    use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

    const TCSANOW: i32 = 0;
    const TIOCGWINSZ: u64 = 0x5413;
    const O_RDWR: i32 = 2;
    const POLLIN: i16 = 0x001;
    const SIGINT: i32 = 2;
    const SIGHUP: i32 = 1;
    const SIGTERM: i32 = 15;
    const SIGWINCH: i32 = 28;

    // termios input flags cleared by cfmakeraw.
    const IGNBRK: u32 = 0o000001;
    const BRKINT: u32 = 0o000002;
    const PARMRK: u32 = 0o000010;
    const ISTRIP: u32 = 0o000040;
    const INLCR: u32 = 0o000100;
    const IGNCR: u32 = 0o000200;
    const ICRNL: u32 = 0o000400;
    const IXON: u32 = 0o002000;
    // output / local flags.
    const OPOST: u32 = 0o000001;
    const ISIG: u32 = 0o000001;
    const ICANON: u32 = 0o000002;
    const ECHO: u32 = 0o000010;
    const ECHONL: u32 = 0o000100;
    const IEXTEN: u32 = 0o100000;
    // control flags.
    const CSIZE: u32 = 0o000060;
    const PARENB: u32 = 0o000400;
    const CS8: u32 = 0o000060;
    // c_cc indices (Linux).
    const VTIME: usize = 5;
    const VMIN: usize = 6;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Termios {
        c_iflag: u32,
        c_oflag: u32,
        c_cflag: u32,
        c_lflag: u32,
        c_line: u8,
        c_cc: [u8; 32],
        c_ispeed: u32,
        c_ospeed: u32,
    }

    #[repr(C)]
    #[derive(Default)]
    struct Winsize {
        ws_row: u16,
        ws_col: u16,
        ws_xpixel: u16,
        ws_ypixel: u16,
    }

    #[repr(C)]
    struct PollFd {
        fd: i32,
        events: i16,
        revents: i16,
    }

    unsafe extern "C" {
        fn open(path: *const u8, flags: i32) -> i32;
        fn close(fd: i32) -> i32;
        fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
        fn write(fd: i32, buf: *const u8, count: usize) -> isize;
        fn tcgetattr(fd: i32, termios: *mut Termios) -> i32;
        fn tcsetattr(fd: i32, actions: i32, termios: *const Termios) -> i32;
        fn ioctl(fd: i32, request: u64, ...) -> i32;
        fn poll(fds: *mut PollFd, nfds: u64, timeout: i32) -> i32;
        fn pipe(fds: *mut i32) -> i32;
        fn signal(signum: i32, handler: usize) -> usize;
    }

    static SIG_WAKE_FD: AtomicI32 = AtomicI32::new(-1);
    static SIG_WINCH: AtomicBool = AtomicBool::new(false);

    extern "C" fn on_wake(_sig: i32) {
        let fd = SIG_WAKE_FD.load(Ordering::Relaxed);
        if fd >= 0 {
            let byte = b"x";
            // write() is async-signal-safe; everything else here is not, so
            // the handler only records and pokes the pipe.
            unsafe { write(fd, byte.as_ptr(), 1) };
        }
    }

    extern "C" fn on_winch(_sig: i32) {
        SIG_WINCH.store(true, Ordering::Relaxed);
        on_wake(_sig);
    }

    fn last_error(what: &str) -> String {
        let errno = std::io::Error::last_os_error();
        format!("{what}: {errno}")
    }

    pub struct Terminal {
        tty_fd: i32,
        wake_fd: i32,
    }

    /// The TUI owns `/dev/tty` directly rather than stdin/stdout: under the
    /// `bind -x` process-substitution seam the helper's stdout is a pipe that
    /// carries the selected command back to Bash, so the interface must be
    /// driven through the controlling terminal (the vim/fzf model).
    impl Terminal {
        pub fn open() -> Result<Self, String> {
            let path = b"/dev/tty\0";
            let tty_fd = unsafe { open(path.as_ptr(), O_RDWR) };
            if tty_fd < 0 {
                return Err(last_error("open /dev/tty"));
            }
            let mut fds = [0_i32; 2];
            if unsafe { pipe(fds.as_mut_ptr()) } != 0 {
                let error = last_error("pipe");
                unsafe { close(tty_fd) };
                return Err(error);
            }
            SIG_WAKE_FD.store(fds[1], Ordering::SeqCst);
            unsafe {
                signal(SIGWINCH, on_winch as *const () as usize);
                signal(SIGTERM, on_wake as *const () as usize);
                signal(SIGINT, on_wake as *const () as usize);
                signal(SIGHUP, on_wake as *const () as usize);
            }
            Ok(Self {
                tty_fd,
                wake_fd: fds[0],
            })
        }

        pub fn enable_raw(&self) -> Result<SavedMode, String> {
            let mut saved = Termios {
                c_iflag: 0,
                c_oflag: 0,
                c_cflag: 0,
                c_lflag: 0,
                c_line: 0,
                c_cc: [0; 32],
                c_ispeed: 0,
                c_ospeed: 0,
            };
            if unsafe { tcgetattr(self.tty_fd, &mut saved) } != 0 {
                return Err(last_error("tcgetattr"));
            }
            let mut raw = saved;
            raw.c_iflag &= !(IGNBRK | BRKINT | PARMRK | ISTRIP | INLCR | IGNCR | ICRNL | IXON);
            raw.c_oflag &= !OPOST;
            raw.c_lflag &= !(ECHO | ECHONL | ICANON | ISIG | IEXTEN);
            raw.c_cflag &= !(CSIZE | PARENB);
            raw.c_cflag |= CS8;
            raw.c_cc[VMIN] = 1;
            raw.c_cc[VTIME] = 0;
            if unsafe { tcsetattr(self.tty_fd, TCSANOW, &raw) } != 0 {
                return Err(last_error("tcsetattr raw"));
            }
            Ok(SavedMode {
                fd: self.tty_fd,
                termios: saved,
                alt_screen: false,
            })
        }

        pub fn size(&self) -> Result<(u16, u16), String> {
            let mut size = Winsize::default();
            if unsafe { ioctl(self.tty_fd, TIOCGWINSZ, &mut size) } != 0 {
                return Err(last_error("TIOCGWINSZ"));
            }
            Ok((size.ws_row, size.ws_col))
        }

        /// Blocks until the tty has input, a signal woke the pipe, or the
        /// timeout expires; reports which.
        pub fn wait_event(&self, timeout_ms: i32) -> Result<Wait, String> {
            let mut fds = [
                PollFd {
                    fd: self.tty_fd,
                    events: POLLIN,
                    revents: 0,
                },
                PollFd {
                    fd: self.wake_fd,
                    events: POLLIN,
                    revents: 0,
                },
            ];
            loop {
                let ready = unsafe { poll(fds.as_mut_ptr(), 2, timeout_ms) };
                if ready < 0 {
                    let error = std::io::Error::last_os_error();
                    if error.kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(last_error("poll"));
                }
                if ready == 0 {
                    return Ok(Wait::Timeout);
                }
                if fds[1].revents & POLLIN != 0 {
                    let mut drain = [0_u8; 64];
                    unsafe { read(self.wake_fd, drain.as_mut_ptr(), drain.len()) };
                    return Ok(Wait::Signal);
                }
                if fds[0].revents & POLLIN != 0 {
                    return Ok(Wait::Input);
                }
            }
        }

        pub fn input_pending(&self) -> bool {
            let mut fds = [PollFd {
                fd: self.tty_fd,
                events: POLLIN,
                revents: 0,
            }];
            let ready = unsafe { poll(fds.as_mut_ptr(), 1, 0) };
            ready > 0 && fds[0].revents & POLLIN != 0
        }

        pub fn read_input(&self, buf: &mut [u8]) -> Result<usize, String> {
            loop {
                let count = unsafe { read(self.tty_fd, buf.as_mut_ptr(), buf.len()) };
                if count < 0 {
                    let error = std::io::Error::last_os_error();
                    if error.kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(last_error("read"));
                }
                return Ok(count as usize);
            }
        }

        pub fn write(&self, bytes: &[u8]) -> Result<(), String> {
            let mut written = 0;
            while written < bytes.len() {
                let count = unsafe {
                    write(
                        self.tty_fd,
                        bytes[written..].as_ptr(),
                        bytes.len() - written,
                    )
                };
                if count < 0 {
                    let error = std::io::Error::last_os_error();
                    if error.kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(last_error("write"));
                }
                written += count as usize;
            }
            Ok(())
        }

        pub fn resize_requested(&self) -> bool {
            SIG_WINCH.swap(false, Ordering::Relaxed)
        }
    }

    impl Drop for Terminal {
        fn drop(&mut self) {
            SIG_WAKE_FD.store(-1, Ordering::SeqCst);
            unsafe {
                signal(SIGWINCH, 0); // SIG_DFL
                signal(SIGTERM, 0);
                signal(SIGINT, 0);
                signal(SIGHUP, 0);
                close(self.wake_fd);
                close(self.wake_fd + 1);
                close(self.tty_fd);
            }
        }
    }

    /// Restores the terminal mode — and, if entered, the alternate screen and
    /// cursor — on every path, including unwinding from a panic.
    pub struct SavedMode {
        fd: i32,
        termios: Termios,
        alt_screen: bool,
    }

    impl SavedMode {
        /// Records that the alternate screen was entered, so `Drop` exits it.
        pub fn entered_alt_screen(&mut self) {
            self.alt_screen = true;
        }
    }

    impl Drop for SavedMode {
        fn drop(&mut self) {
            if self.alt_screen {
                let exit = b"\x1b[?25h\x1b[?1049l";
                unsafe { write(self.fd, exit.as_ptr(), exit.len()) };
            }
            unsafe { tcsetattr(self.fd, TCSANOW, &self.termios) };
        }
    }

    pub enum Wait {
        Input,
        Signal,
        Timeout,
    }
}

#[cfg(target_os = "linux")]
pub use imp::*;

#[cfg(not(target_os = "linux"))]
mod imp {
    /// ADR 0016 scopes the TUI to Linux; other platforms get a clean error
    /// from `run()` rather than a compile failure.
    pub struct Terminal;

    impl Terminal {
        pub fn open() -> Result<Self, String> {
            Err("mbx tui requires Linux (ADR 0016)".to_owned())
        }
    }

    pub struct SavedMode;

    impl SavedMode {
        pub fn entered_alt_screen(&mut self) {}
    }

    pub enum Wait {
        Input,
        Signal,
        Timeout,
    }

    impl Terminal {
        pub fn enable_raw(&self) -> Result<SavedMode, String> {
            Err("mbx tui requires Linux (ADR 0016)".to_owned())
        }

        pub fn size(&self) -> Result<(u16, u16), String> {
            Err("mbx tui requires Linux (ADR 0016)".to_owned())
        }

        pub fn wait_event(&self, _timeout_ms: i32) -> Result<Wait, String> {
            Err("mbx tui requires Linux (ADR 0016)".to_owned())
        }

        pub fn input_pending(&self) -> bool {
            false
        }

        pub fn read_input(&self, _buf: &mut [u8]) -> Result<usize, String> {
            Err("mbx tui requires Linux (ADR 0016)".to_owned())
        }

        pub fn write(&self, _bytes: &[u8]) -> Result<(), String> {
            Err("mbx tui requires Linux (ADR 0016)".to_owned())
        }

        pub fn resize_requested(&self) -> bool {
            false
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub use imp::*;
