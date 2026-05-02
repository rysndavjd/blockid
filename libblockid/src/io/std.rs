pub use std::{
    fs::File,
    io::{Error as IoError, SeekFrom, ErrorKind},
};
use rustix::io::Errno;

use crate::{io::BlockIo, error::Error};

impl BlockIo for File {}

impl From<IoError> for Error<IoError> {
    fn from(e: IoError) -> Self {
        Self::Io(e)
    }
}

impl From<Errno> for Error<IoError> {
    fn from(e: Errno) -> Self {
        match e {
            Errno::NOENT | Errno::NODEV | Errno::NXIO => IoError::from(ErrorKind::NotFound).into(),
            Errno::PERM | Errno::ACCESS => IoError::from(ErrorKind::PermissionDenied).into(),
            Errno::CONNREFUSED => IoError::from(ErrorKind::ConnectionRefused).into(),
            Errno::CONNRESET => IoError::from(ErrorKind::ConnectionReset).into(),
            Errno::CONNABORTED => IoError::from(ErrorKind::ConnectionAborted).into(),
            Errno::NOTCONN => IoError::from(ErrorKind::NotConnected).into(),
            Errno::ADDRINUSE => IoError::from(ErrorKind::AddrInUse).into(),
            Errno::ADDRNOTAVAIL => IoError::from(ErrorKind::AddrNotAvailable).into(),
            Errno::PIPE | Errno::NOLINK => IoError::from(ErrorKind::BrokenPipe).into(),
            Errno::EXIST => IoError::from(ErrorKind::AlreadyExists).into(),
            Errno::INVAL | Errno::BADF | Errno::FAULT => IoError::from(ErrorKind::InvalidInput).into(),
            Errno::ILSEQ | Errno::BADMSG | Errno::PROTO => IoError::from(ErrorKind::InvalidData).into(),
            Errno::TIMEDOUT => IoError::from(ErrorKind::TimedOut).into(),
            Errno::INTR => IoError::from(ErrorKind::Interrupted).into(),
            Errno::NOSYS | Errno::NOTSUP => IoError::from(ErrorKind::Unsupported).into(),
            Errno::NOMEM => IoError::from(ErrorKind::OutOfMemory).into(),
            _ => IoError::from(ErrorKind::Other).into(),
        }
    }
}