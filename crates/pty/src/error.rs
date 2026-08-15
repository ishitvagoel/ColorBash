use std::fmt;
use std::io;

#[derive(Debug)]
pub enum PtyError {
    Open(io::Error),
    Spawn(io::Error),
    Io(io::Error),
    Timeout(Vec<u8>),
    Oversize,
    ChildExited,
}

impl PtyError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Open(_) => "open",
            Self::Spawn(_) => "spawn",
            Self::Io(_) => "io",
            Self::Timeout(_) => "timeout",
            Self::Oversize => "oversize",
            Self::ChildExited => "child_exited",
        }
    }
}

impl fmt::Display for PtyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open(error) => write!(f, "pty open failed: {error}"),
            Self::Spawn(error) => write!(f, "pty spawn failed: {error}"),
            Self::Io(error) => write!(f, "pty i/o failed: {error}"),
            Self::Timeout(captured) => {
                write!(f, "pty deadline exceeded after {} bytes", captured.len())
            }
            Self::Oversize => write!(f, "pty capture exceeded bound"),
            Self::ChildExited => write!(f, "pty child exited"),
        }
    }
}

impl std::error::Error for PtyError {}

impl From<io::Error> for PtyError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
