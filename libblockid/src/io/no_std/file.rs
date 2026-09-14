use embedded_io::{ErrorType as EmbeddedErrorType, Read, Seek, SeekFrom};
use rustix::{
    fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
    fs::{SeekFrom as RustixSeekFrom, seek},
    io::read,
};

use crate::io::{BlockIo, no_std::error::IoError};

#[derive(Debug)]
pub struct File {
    inner: OwnedFd,
}

impl EmbeddedErrorType for File {
    type Error = IoError;
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
            SeekFrom::End(pos) => RustixSeekFrom::End(pos),
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

impl AsRawFd for File {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

impl From<OwnedFd> for File {
    fn from(fd: OwnedFd) -> Self {
        File { inner: fd }
    }
}

impl BlockIo for File {}
