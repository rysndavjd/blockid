mod block;
#[cfg(feature = "os_calls")]
pub mod ioctl;
#[cfg(all(feature = "os_calls", feature = "no_std"))]
pub mod no_std;
#[cfg(feature = "std")]
mod std;

#[cfg(all(not(feature = "os_calls"), feature = "no_std"))]
pub use embedded_io::SeekFrom;

#[cfg(all(feature = "os_calls", feature = "no_std"))]
pub use crate::io::no_std::{Error as IoError, File, SeekFrom, path::PathBuf};
#[cfg(all(not(feature = "os_calls"), feature = "std"))]
pub use crate::io::std::SeekFrom;
#[cfg(all(feature = "os_calls", feature = "std"))]
pub use crate::io::std::{File, IoError, PathBuf, SeekFrom};
use crate::{error::Error, probe::Magic};

/// Trait used to get access to underlying device.
#[cfg(not(feature = "os_calls"))]
pub trait BlockIo: crate::io::block::Io {}

/// Trait used to get access to underlying device with exposed ioctl calls.
#[cfg(feature = "os_calls")]
pub trait BlockIo: crate::io::ioctl::Ioctl {}

/// Reader type used to expose functions provided by [`BlockIo`]
#[derive(Debug)]
pub struct Reader<IO: BlockIo>(IO);

#[allow(dead_code)]
impl<IO: BlockIo> Reader<IO> {
    pub fn new(reader: IO) -> Self {
        Self(reader)
    }

    #[inline]
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error<IO::Error>> {
        self.0.read(buf)
    }

    pub fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), Error<IO::Error>> {
        self.0.seek(SeekFrom::Start(offset))?;
        self.0.read_exact(buf)?;
        Ok(())
    }

    #[inline]
    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error<IO::Error>> {
        self.0.read_exact(buf)
    }

    #[inline]
    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error<IO::Error>> {
        self.0.seek(pos)
    }

    pub fn read_exact_at<const S: usize>(
        &mut self,
        offset: u64,
    ) -> Result<[u8; S], Error<IO::Error>> {
        let mut buf = [0u8; S];
        self.0.seek(SeekFrom::Start(offset))?;
        self.0.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn read_vec_at(&mut self, offset: u64, size: usize) -> Result<Vec<u8>, Error<IO::Error>> {
        let mut buf = vec![0u8; size];
        self.0.seek(SeekFrom::Start(offset))?;
        self.0.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Searches through list of provided magics checking if they exist,
    /// returning the first found magic.
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

            self.read_at(magic.b_offset, &mut buf)?;

            if &buf[..magic.len] == magic.magic {
                return Ok(Some(*magic));
            }
        }

        return Ok(None);
    }

    #[cfg(feature = "os_calls")]
    #[inline]
    pub fn device_size(&self) -> Result<u64, Error<IO::Error>> {
        self.0.device_size()
    }

    #[cfg(feature = "os_calls")]
    #[inline]
    pub fn logical_sector_size(&self) -> Result<u64, Error<IO::Error>> {
        self.0.logical_sector_size()
    }

    #[cfg(feature = "os_calls")]
    #[inline]
    pub fn physical_sector_size(&self) -> Result<u64, Error<IO::Error>> {
        self.0.physical_sector_size()
    }

    #[cfg(all(feature = "os_calls", any(target_os = "linux", target_os = "freebsd")))]
    #[inline]
    pub fn minimum_io_size(&self) -> Result<u64, Error<IO::Error>> {
        self.0.minimum_io_size()
    }

    #[cfg(all(feature = "os_calls", target_os = "linux"))]
    #[inline]
    pub fn optimal_io_size(&self) -> Result<u64, Error<IO::Error>> {
        self.0.optimal_io_size()
    }

    #[cfg(all(feature = "os_calls", any(target_os = "linux", target_os = "freebsd")))]
    #[inline]
    pub fn alignment_offset(&self) -> Result<crate::io::ioctl::AlignmentOffset, Error<IO::Error>> {
        self.0.alignment_offset()
    }
}
