#[cfg(not(feature = "std"))]
pub use embedded_io::SeekFrom;

use crate::std::fmt;
#[cfg(feature = "std")]
pub use crate::std::io::SeekFrom;

pub trait BlockIo {
    type IoError: fmt::Debug + Send + Sync + 'static;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::IoError>;

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::IoError>;

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::IoError>;
}

#[cfg(feature = "std")]
mod io_std {
    use super::BlockIo;

    impl<R: std::io::Read + std::io::Seek> BlockIo for R {
        type IoError = std::io::Error;

        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::IoError> {
            self.read(buf)
        }

        fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::IoError> {
            self.read_exact(buf)
        }

        fn seek(&mut self, pos: std::io::SeekFrom) -> Result<u64, Self::IoError> {
            self.seek(pos)
        }
    }
}

#[cfg(not(feature = "std"))]
mod io_embedded {
    use embedded_io::ErrorKind;

    use super::{BlockIo, fmt};

    impl<R: embedded_io::Read + embedded_io::Seek> BlockIo for R
    where
        R::Error: From<embedded_io::ErrorKind>,
        R::Error: fmt::Debug + Send + Sync + 'static,
    {
        type IoError = R::Error;

        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::IoError> {
            self.read(buf)
        }

        fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::IoError> {
            self.read_exact(buf).map_err(|e| match e {
                embedded_io::ReadExactError::UnexpectedEof => R::Error::from(ErrorKind::NotFound),
                embedded_io::ReadExactError::Other(e) => e,
            })
        }

        fn seek(&mut self, pos: embedded_io::SeekFrom) -> Result<u64, Self::IoError> {
            self.seek(pos)
        }
    }
}

#[derive(Debug)]
pub struct Reader<R: BlockIo>(R);

impl<R: BlockIo> Reader<R> {
    pub fn new(inner: R) -> Self {
        Self(inner)
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, R::IoError> {
        self.0.read(buf)
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), R::IoError> {
        self.0.read_exact(buf)
    }

    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64, R::IoError> {
        self.0.seek(pos)
    }

    pub fn read_exact_at<const S: usize>(&mut self, offset: u64) -> Result<[u8; S], R::IoError> {
        let mut buf = [0u8; S];
        self.0.seek(SeekFrom::Start(offset))?;
        self.0.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), R::IoError> {
        self.0.seek(SeekFrom::Start(offset))?;
        self.0.read_exact(buf)?;
        Ok(())
    }

    pub fn read_vec_at(&mut self, offset: u64, size: usize) -> Result<Vec<u8>, R::IoError> {
        let mut buf = vec![0u8; size];
        self.0.seek(SeekFrom::Start(offset))?;
        self.0.read_exact(&mut buf)?;
        Ok(buf)
    }
}
