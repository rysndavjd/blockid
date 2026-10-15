use embedded_io::{Error as EmbeddedError, ErrorKind as IoErrorKind};
use rustix::io::Errno;

#[derive(Debug)]
pub struct IoError(Errno);

impl core::fmt::Display for IoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "os error {}", self.0.raw_os_error())
    }
}

impl core::error::Error for IoError {}

impl From<Errno> for IoError {
    fn from(e: Errno) -> Self {
        Self(e)
    }
}

impl From<IoError> for crate::error::Error<IoError> {
    fn from(e: IoError) -> Self {
        Self::Io(e)
    }
}

impl From<Errno> for crate::error::Error<IoError> {
    fn from(e: Errno) -> Self {
        Self::Io(IoError(e))
    }
}

impl From<IoErrorKind> for IoError {
    fn from(e: IoErrorKind) -> Self {
        Self(match e {
            IoErrorKind::NotFound => Errno::NODEV,
            IoErrorKind::PermissionDenied => Errno::ACCESS,
            IoErrorKind::ConnectionRefused => Errno::CONNREFUSED,
            IoErrorKind::ConnectionReset => Errno::CONNRESET,
            IoErrorKind::ConnectionAborted => Errno::CONNABORTED,
            IoErrorKind::NotConnected => Errno::NOTCONN,
            IoErrorKind::AddrInUse => Errno::ADDRINUSE,
            IoErrorKind::AddrNotAvailable => Errno::ADDRNOTAVAIL,
            IoErrorKind::BrokenPipe => Errno::PIPE,
            IoErrorKind::AlreadyExists => Errno::EXIST,
            IoErrorKind::InvalidInput => Errno::INVAL,
            IoErrorKind::InvalidData => Errno::ILSEQ,
            IoErrorKind::TimedOut => Errno::TIMEDOUT,
            IoErrorKind::Interrupted => Errno::INTR,
            IoErrorKind::Unsupported => Errno::NOTSUP,
            IoErrorKind::OutOfMemory => Errno::NOMEM,
            _ => Errno::IO,
        })
    }
}

impl From<IoError> for IoErrorKind {
    fn from(e: IoError) -> IoErrorKind {
        e.kind()
    }
}

impl EmbeddedError for IoError {
    fn kind(&self) -> IoErrorKind {
        match self.0 {
            Errno::NOENT | Errno::NODEV | Errno::NXIO => IoErrorKind::NotFound,
            Errno::PERM | Errno::ACCESS => IoErrorKind::PermissionDenied,
            Errno::CONNREFUSED => IoErrorKind::ConnectionRefused,
            Errno::CONNRESET => IoErrorKind::ConnectionReset,
            Errno::CONNABORTED => IoErrorKind::ConnectionAborted,
            Errno::NOTCONN => IoErrorKind::NotConnected,
            Errno::ADDRINUSE => IoErrorKind::AddrInUse,
            Errno::ADDRNOTAVAIL => IoErrorKind::AddrNotAvailable,
            Errno::PIPE | Errno::NOLINK => IoErrorKind::BrokenPipe,
            Errno::EXIST => IoErrorKind::AlreadyExists,
            Errno::INVAL | Errno::BADF | Errno::FAULT => IoErrorKind::InvalidInput,
            Errno::ILSEQ | Errno::BADMSG | Errno::PROTO => IoErrorKind::InvalidData,
            Errno::TIMEDOUT => IoErrorKind::TimedOut,
            Errno::INTR => IoErrorKind::Interrupted,
            Errno::NOSYS | Errno::NOTSUP => IoErrorKind::Unsupported,
            Errno::NOMEM => IoErrorKind::OutOfMemory,
            _ => IoErrorKind::Other,
        }
    }
}
