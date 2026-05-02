pub use embedded_io::SeekFrom;
use embedded_io::{Error as EmbeddedError, ErrorKind, ErrorType as EmbeddedErrorType, Read, Seek};
use rustix::{
    fd::{AsFd, BorrowedFd, OwnedFd},
    fs::{Mode, OFlags, SeekFrom as RustixSeekFrom, open, seek},
    io::{Errno, read},
};

use crate::io::{BlockIo, block::Io, path::SysPath};

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

impl From<Error> for crate::error::Error<Error> {
    fn from(e: Error) -> Self {
        Self::Io(e)
    }
}

impl From<Errno> for crate::error::Error<Error> {
    fn from(e: Errno) -> Self {
        Self::Io(Error(e))
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

impl From<Error> for embedded_io::ErrorKind {
    fn from(e: Error) -> embedded_io::ErrorKind {
        e.kind()
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

impl File {
    pub fn open<P: SysPath>(path: P) -> Result<File, Error> {
        let fd = open(path.as_ref().as_bytes(), OFlags::RDONLY, Mode::empty())?;

        Ok(Self { inner: fd })
    }
}

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
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        let new_pos = match pos {
            SeekFrom::Start(pos) => RustixSeekFrom::Start(pos),
            SeekFrom::End(pos) => RustixSeekFrom::Current(pos),
            SeekFrom::Current(pos) => RustixSeekFrom::Current(pos),
        };

        let ret = seek(&self.inner, new_pos)?;
        Ok(ret)
    }
}

impl AsFd for File {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.inner.as_fd()
    }
}

impl From<OwnedFd> for File {
    fn from(fd: OwnedFd) -> Self {
        File { inner: fd }
    }
}

impl BlockIo for File {}
