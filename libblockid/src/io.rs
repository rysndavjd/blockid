#[cfg(feature = "os_calls")]
pub mod ioctl;
#[cfg(feature = "no_std")]
mod path;
#[cfg(all(feature = "no_std", target_family = "unix"))]
mod unix;
#[cfg(all(feature = "no_std", target_family = "windows"))]
mod windows;

use crate::{error::Error, probe::Magic};

pub trait BlockIo: crate::std::fmt::Debug {
    type Error: crate::std::fmt::Debug;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error>;

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error>;
}

#[cfg(feature = "std")]
pub use std::io::SeekFrom;

#[cfg(feature = "std")]
impl<R: std::io::Read + std::io::Seek + std::fmt::Debug> BlockIo for R {
    type Error = std::io::Error;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.read(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.read_exact(buf)
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        self.seek(pos)
    }
}

#[cfg(feature = "no_std")]
pub use embedded_io::{ErrorKind, SeekFrom};

#[cfg(feature = "no_std")]
impl<
    E: From<embedded_io::ErrorKind> + core::fmt::Debug,
    R: embedded_io::Read + embedded_io::Seek<Error = E> + core::fmt::Debug,
> BlockIo for R
where
    embedded_io::ErrorKind: core::convert::From<E>,
{
    type Error = R::Error;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.read(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.read_exact(buf).map_err(|e| match e {
            embedded_io::ReadExactError::UnexpectedEof => ErrorKind::InvalidInput.into(),
            embedded_io::ReadExactError::Other(e) => e,
        })
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        self.seek(pos)
    }
}

#[derive(Debug)]
pub struct Reader<IO: BlockIo>(IO);

impl<IO: BlockIo> Reader<IO> {
    pub fn new(reader: IO) -> Self {
        Self(reader)
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, IO::Error> {
        self.0.read(buf)
    }

    pub fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), IO::Error> {
        self.0.seek(SeekFrom::Start(offset))?;
        self.0.read_exact(buf)?;
        Ok(())
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), IO::Error> {
        self.0.read_exact(buf)
    }

    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64, IO::Error> {
        self.0.seek(pos)
    }

    pub fn read_exact_at<const S: usize>(&mut self, offset: u64) -> Result<[u8; S], IO::Error> {
        let mut buf = [0u8; S];
        self.0.seek(SeekFrom::Start(offset))?;
        self.0.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn read_vec_at(&mut self, offset: u64, size: usize) -> Result<Vec<u8>, IO::Error> {
        let mut buf = vec![0u8; size];
        self.0.seek(SeekFrom::Start(offset))?;
        self.0.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn get_magic(
        &mut self,
        magics: &'static [Magic],
    ) -> Result<Option<Magic>, Error<IO::Error>> {
        let mut buf = [0u8; 16];

        for magic in magics {
            debug_assert!(
                magic.len <= buf.len(),
                "Magic should not be greater then `buf`"
            );

            self.read_at(magic.b_offset, &mut buf).map_err(Error::Io)?;

            if &buf[..magic.len] == magic.magic {
                return Ok(Some(*magic));
            }
        }

        return Ok(None);
    }
}

#[cfg(feature = "std")]
pub use std::{fs::File, io::Error as IoError};

#[cfg(all(feature = "no_std", target_family = "unix"))]
pub use crate::io::unix::{Error as IoError, File};
