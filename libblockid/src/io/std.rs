pub use std::io::SeekFrom;
#[cfg(feature = "os_calls")]
pub use std::{
    fs::File,
    io::{Error as IoError, ErrorKind as IoErrorKind},
    path::{Path, PathBuf},
};

#[cfg(feature = "os_calls")]
mod file {
    use std::{
        fs::File,
        io::{Error as IoError, ErrorKind},
    };

    use rustix::io::Errno;

    use crate::{
        error::{Error, IdToDiskError, PartToDiskError},
        io::BlockIo,
    };

    impl BlockIo for File {}

    impl From<IoError> for Error<IoError> {
        fn from(e: IoError) -> Self {
            Self::Io(e)
        }
    }

    fn errno_to_ioerror(e: Errno) -> IoError {
        match e {
            Errno::NOENT | Errno::NODEV | Errno::NXIO => IoError::from(ErrorKind::NotFound),
            Errno::PERM | Errno::ACCESS => IoError::from(ErrorKind::PermissionDenied),
            Errno::CONNREFUSED => IoError::from(ErrorKind::ConnectionRefused),
            Errno::CONNRESET => IoError::from(ErrorKind::ConnectionReset),
            Errno::CONNABORTED => IoError::from(ErrorKind::ConnectionAborted),
            Errno::NOTCONN => IoError::from(ErrorKind::NotConnected),
            Errno::ADDRINUSE => IoError::from(ErrorKind::AddrInUse),
            Errno::ADDRNOTAVAIL => IoError::from(ErrorKind::AddrNotAvailable),
            Errno::PIPE | Errno::NOLINK => IoError::from(ErrorKind::BrokenPipe),
            Errno::EXIST => IoError::from(ErrorKind::AlreadyExists),
            Errno::INVAL | Errno::BADF | Errno::FAULT => IoError::from(ErrorKind::InvalidInput),
            Errno::ILSEQ | Errno::BADMSG | Errno::PROTO => IoError::from(ErrorKind::InvalidData),
            Errno::TIMEDOUT => IoError::from(ErrorKind::TimedOut),
            Errno::INTR => IoError::from(ErrorKind::Interrupted),
            Errno::NOSYS | Errno::NOTSUP => IoError::from(ErrorKind::Unsupported),
            Errno::NOMEM => IoError::from(ErrorKind::OutOfMemory),
            _ => IoError::from(ErrorKind::Other),
        }
    }

    impl From<Errno> for Error<IoError> {
        fn from(e: Errno) -> Self {
            Error::Io(errno_to_ioerror(e))
        }
    }

    impl From<Errno> for PartToDiskError<IoError> {
        fn from(e: Errno) -> Self {
            PartToDiskError::Io(errno_to_ioerror(e))
        }
    }

    impl From<Errno> for IdToDiskError<IoError> {
        fn from(e: Errno) -> Self {
            IdToDiskError::Io(errno_to_ioerror(e))
        }
    }
}
