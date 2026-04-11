use crate::{
    filesystem::{exfat::ExFatError, ext::ExtError, luks::LuksError, vfat::VFatError},
    io::BlockIo,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuilderError {}

#[non_exhaustive]
#[derive(Debug)]
pub enum Error<IO: BlockIo> {
    Io(IO::Error),
    Luks(LuksError),
    ExFat(ExFatError),
    Ext(ExtError),
    VFat(VFatError),
    ProbesExhausted,
}

impl<IO: BlockIo> Error<IO> {}

impl<IO: BlockIo> Error<IO> {
    pub(crate) fn io(e: IO::Error) -> Self {
        Self::Io(e)
    }
}
// #[derive(Clone, Copy, Debug, Eq, PartialEq)]

// impl crate::std::error::Error for Error {}
