use crate::WinSize;
use crate::error::PtyError;
use std::ffi::CStr;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
#[cfg(target_os = "macos")]
use std::os::raw::c_uint;
use std::os::raw::{c_char, c_int, c_short, c_void};
use std::path::PathBuf;
use std::time::Instant;

const O_RDWR: c_int = 2;
#[cfg(target_os = "macos")]
pub(crate) const O_NOCTTY: c_int = 0x20000;
#[cfg(not(target_os = "macos"))]
pub(crate) const O_NOCTTY: c_int = 0o400;
// Darwin bsd/sys/fcntl.h: O_CLOEXEC 0x01000000 when __DARWIN_C_LEVEL >= 200809L
#[cfg(target_os = "macos")]
const O_CLOEXEC: c_int = 0x0100_0000;
// Linux/glibc fcntl.h: O_CLOEXEC 02000000
#[cfg(not(target_os = "macos"))]
const O_CLOEXEC: c_int = 0o2000000;
const O_NONBLOCK: c_int = 0o4000;
const F_GETFL: c_int = 3;
const F_SETFL: c_int = 4;
const POLLIN: c_short = 0x001;
const POLLOUT: c_short = 0x004;
const POLLERR: c_short = 0x008;
const POLLHUP: c_short = 0x010;

#[cfg(target_os = "linux")]
const TIOCSCTTY: usize = 0x540E;
#[cfg(target_os = "linux")]
const TIOCGWINSZ: usize = 0x5413;
#[cfg(target_os = "linux")]
const TIOCSWINSZ: usize = 0x5414;

#[cfg(target_os = "macos")]
const TIOCSCTTY: usize = 0x2000_7461;
#[cfg(target_os = "macos")]
const TIOCGWINSZ: usize = 0x4008_7468;
#[cfg(target_os = "macos")]
const TIOCSWINSZ: usize = 0x8008_7467;

#[repr(C)]
struct PollFd {
    fd: c_int,
    events: c_short,
    revents: c_short,
}

#[repr(C)]
struct KernelWinSize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

#[cfg(target_os = "linux")]
#[repr(C)]
pub struct Termios {
    pub c_iflag: u32,
    pub c_oflag: u32,
    pub c_cflag: u32,
    pub c_lflag: u32,
    pub c_line: u8,
    pub c_cc: [u8; 32],
    pub c_ispeed: u32,
    pub c_ospeed: u32,
}

#[cfg(target_os = "macos")]
#[repr(C)]
pub struct Termios {
    pub c_iflag: u64,
    pub c_oflag: u64,
    pub c_cflag: u64,
    pub c_lflag: u64,
    pub c_cc: [u8; 20],
    pub c_ispeed: u64,
    pub c_ospeed: u64,
}

// Darwin bsd/sys/poll.h typedef unsigned int nfds_t; Linux x86_64 uses unsigned long.
#[cfg(not(target_os = "macos"))]
type PollNfds = u64;
#[cfg(target_os = "macos")]
type PollNfds = c_uint;

unsafe extern "C" {
    fn posix_openpt(flags: c_int) -> c_int;
    fn grantpt(fd: c_int) -> c_int;
    fn unlockpt(fd: c_int) -> c_int;
    // POSIX.1-2001; macOS grantpt(3) since 10.13.4. Fixed buffer avoids ptsname().
    fn ptsname_r(fd: c_int, buf: *mut c_char, buflen: usize) -> c_int;
    fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    fn ioctl(fd: c_int, request: usize, ...) -> c_int;
    fn setsid() -> c_int;
    fn poll(fds: *mut PollFd, nfds: PollNfds, timeout: c_int) -> c_int;
    fn tcgetattr(fd: c_int, termios_p: *mut Termios) -> c_int;
    fn kill(pid: c_int, sig: c_int) -> c_int;
}

pub fn open_master() -> Result<OwnedFd, PtyError> {
    let fd = unsafe { posix_openpt(O_RDWR | O_NOCTTY | O_CLOEXEC) };
    if fd < 0 {
        return Err(PtyError::Open(io::Error::last_os_error()));
    }
    let master = unsafe { OwnedFd::from_raw_fd(fd) };
    if unsafe { grantpt(master.as_raw_fd()) } != 0 {
        return Err(PtyError::Open(io::Error::last_os_error()));
    }
    if unsafe { unlockpt(master.as_raw_fd()) } != 0 {
        return Err(PtyError::Open(io::Error::last_os_error()));
    }
    set_nonblocking(master.as_raw_fd()).map_err(PtyError::Open)?;
    Ok(master)
}

pub fn slave_path(master: &OwnedFd) -> Result<PathBuf, PtyError> {
    let mut buf = [0u8; 128];
    let rc = unsafe {
        ptsname_r(
            master.as_raw_fd(),
            buf.as_mut_ptr().cast::<c_char>(),
            buf.len(),
        )
    };
    if rc != 0 {
        return Err(PtyError::Open(io::Error::from_raw_os_error(rc)));
    }
    let name = CStr::from_bytes_until_nul(&buf)
        .map_err(|_| PtyError::Open(io::Error::other("ptsname was not NUL-terminated")))?;
    Ok(PathBuf::from(name.to_string_lossy().into_owned()))
}

pub fn set_nonblocking(fd: c_int) -> io::Result<()> {
    let flags = unsafe { fcntl(fd, F_GETFL, 0) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { fcntl(fd, F_SETFL, flags | O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub fn become_session_leader() -> io::Result<()> {
    if unsafe { setsid() } < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn set_controlling_tty(fd: c_int) -> io::Result<()> {
    if unsafe { ioctl(fd, TIOCSCTTY, std::ptr::null_mut::<c_void>()) } < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn set_winsize(fd: c_int, size: WinSize) -> io::Result<()> {
    let winsize = KernelWinSize {
        ws_row: size.rows,
        ws_col: size.cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    if unsafe { ioctl(fd, TIOCSWINSZ, &winsize) } < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn get_winsize(fd: c_int) -> io::Result<WinSize> {
    let mut winsize = KernelWinSize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    if unsafe { ioctl(fd, TIOCGWINSZ, &mut winsize) } < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(WinSize {
            rows: winsize.ws_row,
            cols: winsize.ws_col,
        })
    }
}

pub fn get_termios(fd: c_int) -> io::Result<Termios> {
    let mut termios = unsafe { std::mem::zeroed() };
    if unsafe { tcgetattr(fd, &mut termios) } < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(termios)
    }
}

pub fn wait_ready(fd: c_int, write: bool, deadline: Instant) -> Result<bool, PtyError> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    let timeout_ms = i32::try_from(
        remaining
            .as_millis()
            .saturating_add(u128::from(remaining.subsec_nanos() > 0)),
    )
    .unwrap_or(i32::MAX);
    let mut pollfd = PollFd {
        fd,
        events: if write { POLLOUT } else { POLLIN },
        revents: 0,
    };
    let rc = unsafe { poll(&mut pollfd, 1, timeout_ms) };
    if rc < 0 {
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::Interrupted {
            return Ok(false);
        }
        return Err(PtyError::Io(error));
    }
    if rc == 0 {
        return Err(PtyError::Timeout(Vec::new()));
    }
    if pollfd.revents & (POLLERR | POLLHUP) != 0 && pollfd.revents & POLLIN == 0 {
        return Err(PtyError::ChildExited);
    }
    Ok(true)
}

pub fn send_signal(pid: u32, signal: i32) -> io::Result<()> {
    let pid = c_int::try_from(pid).map_err(|_| io::Error::other("child pid out of range"))?;
    if unsafe { kill(pid, signal) } < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn send_group_signal(pid: u32, signal: i32) -> io::Result<()> {
    let pid = c_int::try_from(pid).map_err(|_| io::Error::other("child pid out of range"))?;
    if unsafe { kill(-pid, signal) } < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod platform_constants {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn linux_open_flags_match_glibc() {
        assert_eq!(O_CLOEXEC, 0o2000000);
        assert_eq!(O_NOCTTY, 0o400);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn darwin_open_flags_match_fcntl_h() {
        assert_eq!(O_CLOEXEC, 0x0100_0000);
        assert_eq!(O_NOCTTY, 0x20000);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn linux_poll_nfds_matches_unsigned_long() {
        assert_eq!(std::mem::size_of::<PollNfds>(), std::mem::size_of::<u64>());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn darwin_poll_nfds_matches_unsigned_int() {
        assert_eq!(
            std::mem::size_of::<PollNfds>(),
            std::mem::size_of::<c_uint>()
        );
    }

    #[test]
    fn slave_path_is_bounded_and_nul_terminated() {
        let master = open_master().expect("open master");
        let path = slave_path(&master).expect("slave path");
        let rendered = path.to_string_lossy();
        assert!(rendered.starts_with("/dev/"));
        assert!(rendered.len() < 128);
    }
}
