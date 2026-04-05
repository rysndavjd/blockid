use crate::{filesystem::ext::ExtError, io::BlockIo};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuilderError {}

#[derive(Debug)]
pub struct Error<IO: BlockIo + Sized>(pub(crate) ErrorKind<IO>);

impl<IO: BlockIo> From<ErrorKind<IO>> for Error<IO> {
    fn from(v: ErrorKind<IO>) -> Self {
        Self(v)
    }
}

impl<IO: BlockIo> From<ErrorKind<IO>> for IO::IoError {
    fn from(value: ErrorKind<IO>) -> Self {
        Self()
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub enum ErrorKind<IO: BlockIo> {
    IoError(IO::IoError),
    ExtError(ExtError),
    // VFatError(VFatError),
    ProbesExhausted,
}

// #[derive(Clone, Copy, Debug, Eq, PartialEq)]

// impl crate::std::error::Error for Error {}
