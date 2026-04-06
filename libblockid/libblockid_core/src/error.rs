use crate::{
    filesystem::{exfat::ExFatError, ext::ExtError, vfat::VFatError},
    io::BlockIo,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuilderError {}

#[derive(Debug)]
pub struct Error<IO: BlockIo>(pub(crate) ErrorKind<IO>);

impl<IO: BlockIo> Error<IO> {
    pub fn io(e: IO::Error) -> Self {
        Error(ErrorKind::IoError(e))
    }
}

impl<IO: BlockIo> From<ErrorKind<IO>> for Error<IO> {
    fn from(e: ErrorKind<IO>) -> Self {
        Self(e)
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub enum ErrorKind<IO: BlockIo> {
    IoError(IO::Error),
    ExFatError(ExFatError),
    ExtError(ExtError),
    VFatError(VFatError),
    MagicCannotBeEmpty,
    ProbesExhausted,
}

// #[derive(Clone, Copy, Debug, Eq, PartialEq)]

// impl crate::std::error::Error for Error {}
