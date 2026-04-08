#[cfg(feature = "std")]
pub use std::{
    fs::File,
    io::{Error, ErrorKind},
};

#[cfg(all(not(feature = "std"), target_family = "unix"))]
pub use impl_unix::{Error, ErrorKind, File};

#[cfg(all(not(feature = "std"), target_family = "unix"))]
mod impl_unix {
    pub use embedded_io::ErrorKind;
    use embedded_io::{
        Error as EmbeddedError, ErrorType as EmbeddedErrorType, Read, Seek,
        SeekFrom as EmbeddedSeekFrom,
    };
    use rustix::{
        fd::OwnedFd,
        fs::{SeekFrom as RustixSeekFrom, seek},
        io::{Errno, read},
    };

    #[derive(Debug)]
    pub struct Error(Errno);

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "os error {}", self.0.raw_os_error())
        }
    }

    impl core::error::Error for Error {}

    impl From<Errno> for Error {
        fn from(e: Errno) -> Self {
            Self(e)
        }
    }

    impl From<ErrorKind> for Error {
        fn from(e: ErrorKind) -> Self {
            Self(match e {
                ErrorKind::NotFound => Errno::NODEV,
                ErrorKind::PermissionDenied => Errno::ACCESS,
                ErrorKind::ConnectionRefused => Errno::CONNREFUSED,
                ErrorKind::ConnectionReset => Errno::CONNRESET,
                ErrorKind::ConnectionAborted => Errno::CONNABORTED,
                ErrorKind::NotConnected => Errno::NOTCONN,
                ErrorKind::AddrInUse => Errno::ADDRINUSE,
                ErrorKind::AddrNotAvailable => Errno::ADDRNOTAVAIL,
                ErrorKind::BrokenPipe => Errno::PIPE,
                ErrorKind::AlreadyExists => Errno::EXIST,
                ErrorKind::InvalidInput => Errno::INVAL,
                ErrorKind::InvalidData => Errno::ILSEQ,
                ErrorKind::TimedOut => Errno::TIMEDOUT,
                ErrorKind::Interrupted => Errno::INTR,
                ErrorKind::Unsupported => Errno::NOTSUP,
                ErrorKind::OutOfMemory => Errno::NOMEM,
                _ => Errno::IO,
            })
        }
    }

    impl EmbeddedError for Error {
        fn kind(&self) -> ErrorKind {
            match self.0 {
                Errno::NOENT | Errno::NODEV | Errno::NXIO => ErrorKind::NotFound,
                Errno::PERM | Errno::ACCESS => ErrorKind::PermissionDenied,
                Errno::CONNREFUSED => ErrorKind::ConnectionRefused,
                Errno::CONNRESET => ErrorKind::ConnectionReset,
                Errno::CONNABORTED => ErrorKind::ConnectionAborted,
                Errno::NOTCONN => ErrorKind::NotConnected,
                Errno::ADDRINUSE => ErrorKind::AddrInUse,
                Errno::ADDRNOTAVAIL => ErrorKind::AddrNotAvailable,
                Errno::PIPE | Errno::NOLINK => ErrorKind::BrokenPipe,
                Errno::EXIST => ErrorKind::AlreadyExists,
                Errno::INVAL | Errno::BADF | Errno::FAULT => ErrorKind::InvalidInput,
                Errno::ILSEQ | Errno::BADMSG | Errno::PROTO => ErrorKind::InvalidData,
                Errno::TIMEDOUT => ErrorKind::TimedOut,
                Errno::INTR => ErrorKind::Interrupted,
                Errno::NOSYS | Errno::NOTSUP => ErrorKind::Unsupported,
                Errno::NOMEM => ErrorKind::OutOfMemory,
                _ => ErrorKind::Other,
            }
        }
    }

    #[derive(Debug)]
    pub struct File {
        inner: OwnedFd,
    }

    impl File {}

    impl EmbeddedErrorType for File {
        type Error = Error;
    }

    impl Read for File {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            let out = read(&self.inner, buf)?;
            Ok(out)
        }
    }

    impl Seek for File {
        fn seek(&mut self, pos: EmbeddedSeekFrom) -> Result<u64, Self::Error> {
            let new_pos = match pos {
                EmbeddedSeekFrom::Start(pos) => RustixSeekFrom::Start(pos),
                EmbeddedSeekFrom::End(pos) => RustixSeekFrom::Current(pos),
                EmbeddedSeekFrom::Current(pos) => RustixSeekFrom::Current(pos),
            };

            let ret = seek(&self.inner, new_pos)?;
            Ok(ret)
        }
    }
}

#[cfg(all(not(feature = "std"), target_family = "windows"))]
mod windows {}
