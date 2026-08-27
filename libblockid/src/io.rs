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
use crate::{error::Error, probe::Magic, std::ops::Range};

/// Trait used to get access to underlying device.
#[cfg(not(feature = "os_calls"))]
pub trait BlockIo: crate::io::block::Io {}

/// Trait used to get access to underlying device with exposed ioctl calls.
#[cfg(feature = "os_calls")]
pub trait BlockIo: crate::io::ioctl::Ioctl {}

/// Reader type used to expose functions provided by [`BlockIo`]
#[derive(Debug)]
pub struct Reader<IO: BlockIo> {
    /// Underlying IO inteface.
    io: IO,
    #[cfg(feature = "os_calls")]
    os_calls: bool,
}

#[allow(dead_code)]
impl<IO: BlockIo> Reader<IO> {
    #[cfg(feature = "os_calls")]
    pub const fn new(io: IO, os_calls: bool) -> Self {
        Self { io, os_calls }
    }

    #[cfg(not(feature = "os_calls"))]
    pub const fn new(io: IO) -> Self {
        Self { io }
    }

    #[inline]
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error<IO::Error>> {
        self.io.read(buf)
    }

    pub fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), Error<IO::Error>> {
        self.io.seek(SeekFrom::Start(offset))?;
        self.io.read_exact(buf)?;
        Ok(())
    }

    #[inline]
    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error<IO::Error>> {
        self.io.read_exact(buf)
    }

    #[inline]
    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error<IO::Error>> {
        self.io.seek(pos)
    }

    pub fn read_exact_at<const S: usize>(
        &mut self,
        offset: u64,
    ) -> Result<[u8; S], Error<IO::Error>> {
        let mut buf = [0u8; S];
        self.io.seek(SeekFrom::Start(offset))?;
        self.io.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn read_vec_at(&mut self, offset: u64, size: usize) -> Result<Vec<u8>, Error<IO::Error>> {
        let mut buf = vec![0u8; size];
        self.io.seek(SeekFrom::Start(offset))?;
        self.io.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn read_at_exclude(
        &mut self,
        offset: u64,
        size: usize,
        exclude: Range<usize>,
    ) -> Result<Vec<u8>, Error<IO::Error>> {
        let mut buf = self.read_vec_at(offset, size)?;
        if exclude.end > size {
            return Err(Error::RangeEndExceedsGivenSize);
        }
        buf[exclude].fill(0);

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
                magic.bytes.len() <= buf.len(),
                "Magic should not be greater then `buf`"
            );

            self.read_at(magic.offset, &mut buf)?;

            if &buf[..(magic.bytes.len())] == magic.bytes {
                return Ok(Some(*magic));
            }
        }

        return Ok(None);
    }

    /// Returns `true` if the probed object is a block device that `os_calls`
    /// can retrieve information on from the OS, otherwise returns `false`,
    /// meaning the probed object is probably a regular file.
    #[cfg(feature = "os_calls")]
    #[inline]
    pub(crate) fn os_calls(&self) -> bool {
        self.os_calls
    }

    #[cfg(feature = "os_calls")]
    #[inline]
    pub fn device_size(&self) -> Result<u64, Error<IO::Error>> {
        self.io.device_size()
    }

    #[cfg(feature = "os_calls")]
    #[inline]
    pub fn logical_sector_size(&self) -> Result<u64, Error<IO::Error>> {
        self.io.logical_sector_size()
    }

    #[cfg(feature = "os_calls")]
    #[inline]
    pub fn physical_sector_size(&self) -> Result<u64, Error<IO::Error>> {
        self.io.physical_sector_size()
    }

    #[cfg(all(feature = "os_calls", any(target_os = "linux", target_os = "freebsd")))]
    #[inline]
    pub fn minimum_io_size(&self) -> Result<u64, Error<IO::Error>> {
        self.io.minimum_io_size()
    }

    #[cfg(all(feature = "os_calls", target_os = "linux"))]
    #[inline]
    pub fn optimal_io_size(&self) -> Result<u64, Error<IO::Error>> {
        self.io.optimal_io_size()
    }

    #[cfg(all(feature = "os_calls", any(target_os = "linux", target_os = "freebsd")))]
    #[inline]
    pub fn alignment_offset(&self) -> Result<crate::io::ioctl::AlignmentOffset, Error<IO::Error>> {
        self.io.alignment_offset()
    }
}
