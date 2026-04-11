use libblockid_core::{BlockIo, Error as CoreError, ExFatError, ExtError, LuksError, VFatError};

#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    Io(crate::io::Error),
    Luks(LuksError),
    ExFat(ExFatError),
    Ext(ExtError),
    VFat(VFatError),
    ProbesExhausted,
}

impl From<rustix::io::Errno> for Error {
    fn from(e: rustix::io::Errno) -> Self {
        Error::Io(e.into())
    }
}

impl From<crate::io::Error> for Error {
    fn from(e: crate::io::Error) -> Self {
        Self::Io(e)
    }
}

impl<IO: BlockIo<Error = crate::io::Error>> From<CoreError<IO>> for Error {
    fn from(e: CoreError<IO>) -> Self {
        match e {
            CoreError::Io(e) => Self::Io(e),
            CoreError::Luks(e) => Self::Luks(e),
            CoreError::ExFat(e) => Self::ExFat(e),
            CoreError::Ext(e) => Self::Ext(e),
            CoreError::VFat(e) => Self::VFat(e),
            CoreError::ProbesExhausted => Self::ProbesExhausted,
            _ => unreachable!(),
        }
    }
}
