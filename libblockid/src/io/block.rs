#[cfg(feature = "std")]
pub use std::io::SeekFrom;

#[cfg(feature = "no_std")]
pub use embedded_io::SeekFrom;

pub trait Io: crate::std::fmt::Debug {
    type Error: crate::std::fmt::Debug;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error>;

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error>;
}

#[cfg(feature = "std")]
impl<R: std::io::Read + std::io::Seek + std::fmt::Debug> Io for R {
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
impl<
    E: From<embedded_io::ErrorKind> + core::fmt::Debug,
    R: embedded_io::Read + embedded_io::Seek<Error = E> + core::fmt::Debug,
> Io for R
where
    embedded_io::ErrorKind: core::convert::From<E>,
{
    type Error = R::Error;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.read(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.read_exact(buf).map_err(|e| match e {
            embedded_io::ReadExactError::UnexpectedEof => {
                embedded_io::ErrorKind::InvalidInput.into()
            }
            embedded_io::ReadExactError::Other(e) => e,
        })
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        self.seek(pos)
    }
}
