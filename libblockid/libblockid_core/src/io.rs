#[cfg(not(feature = "std"))]
pub use embedded_io::{Error, ErrorKind, SeekFrom};

use crate::std::fmt;
#[cfg(feature = "std")]
pub use crate::std::io::SeekFrom;

pub trait BlockIo: fmt::Debug {
    type Error: fmt::Debug;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error>;

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error>;
}

#[cfg(feature = "std")]
impl<R: std::io::Read + std::io::Seek + fmt::Debug> BlockIo for R {
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

#[cfg(not(feature = "std"))]
impl<R: embedded_io::Read + embedded_io::Seek<Error = embedded_io::ErrorKind> + fmt::Debug> BlockIo
    for R
{
    type Error = R::Error;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.read(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.read_exact(buf).map_err(|e| match e {
            embedded_io::ReadExactError::UnexpectedEof => ErrorKind::InvalidInput,
            embedded_io::ReadExactError::Other(e) => e.kind(),
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
}
