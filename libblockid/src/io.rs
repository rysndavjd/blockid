mod block;
#[cfg(feature = "os_calls")]
mod ioctl;
#[cfg(feature = "no_std")]
mod path;
#[cfg(feature = "std")]
mod std;
#[cfg(all(feature = "os_calls", feature = "no_std", target_family = "unix"))]
mod unix;

#[cfg(feature = "no_std")]
pub use embedded_io::SeekFrom;

#[cfg(feature = "std")]
pub use crate::io::std::{File, SeekFrom, IoError};
#[cfg(all(feature = "os_calls", feature = "no_std", target_family = "unix"))]
pub use crate::io::unix::{File};
use crate::{error::Error, probe::Magic};

#[cfg(not(feature = "os_calls"))]
pub trait BlockIo: crate::io::block::Io {}

#[cfg(feature = "os_calls")]
pub trait BlockIo: crate::io::ioctl::Ioctl {}

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

    #[cfg(feature = "os_calls")]
    pub fn logical_sector_size(&mut self) -> Result<u64, Error<IO::Error>> {
        self.0.logical_sector_size()
    }

    #[cfg(feature = "os_calls")]
    pub fn physical_sector_size(&mut self) -> Result<u64, Error<IO::Error>> {
        self.0.physical_sector_size()
    }

    #[cfg(all(
        feature = "os_calls",
        any(target_os = "linux", target_os = "freebsd")
    ))]
    pub fn minimum_io_size(&mut self) -> Result<u64, Error<IO::Error>> {
        self.0.minimum_io_size()
    }

    #[cfg(all(feature = "os_calls", target_os = "linux"))]
    pub fn optimal_io_size(&mut self) -> Result<u64, Error<IO::Error>> {
        self.0.optimal_io_size()
    }

    #[cfg(all(feature = "os_calls", any(target_os = "linux", target_os = "freebsd")))]
    pub fn alignment_offset(
        &mut self,
    ) -> Result<crate::io::ioctl::AlignmentOffset, Error<IO::Error>> {
        self.0.alignment_offset()
    }
}
