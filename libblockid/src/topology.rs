#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum AlignmentOffset {
    Misaligned,
    Offset(u64),
}

#[derive(Debug)]
pub struct TopologyInfo {
    pub(crate) logical_sector_size: u64,
    pub(crate) physical_sector_size: u64,
    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "windows"))]
    pub(crate) minimum_io_size: u64,
    #[cfg(target_os = "linux")]
    pub(crate) optimal_io_size: u64,
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    pub(crate) alignment_offset: AlignmentOffset,
}

impl TopologyInfo {
    pub fn logical_sector_size(&self) -> u64 {
        self.logical_sector_size
    }

    pub fn physical_sector_size(&self) -> u64 {
        self.physical_sector_size
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "windows"))]
    pub fn minimum_io_size(&self) -> u64 {
        self.minimum_io_size
    }

    #[cfg(target_os = "linux")]
    pub fn optimal_io_size(&self) -> u64 {
        self.optimal_io_size
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    pub fn alignment_offset(&self) -> AlignmentOffset {
        self.alignment_offset
    }
}

/* Note:
 * The rustix::ioctl::opcode::read function seems to calculate different values
 * on different systems. For example:
 *
 *   read::<u32>(b'd', 24) == 2147771416 on Linux
 *   read::<u32>(b'd', 24) == 1074029592 on macOS
 */

use crate::io::{File, IoError};

pub fn logical_sector_size(file: &mut File) -> Result<u64, IoError> {
    #[cfg(target_os = "freebsd")]
    todo!();

    #[cfg(target_os = "linux")]
    {
        let sz = rustix::fs::ioctl_blksszget(file)?;
        Ok(sz.into())
    }

    #[cfg(target_os = "macos")]
    {
        let sz = macos::ioctl_dkiocgetblocksize(file)?;
        Ok(sz.into())
    }

    #[cfg(target_os = "windows")]
    todo!();
}

pub fn physical_sector_size(file: &mut File) -> Result<u64, IoError> {
    #[cfg(target_os = "freebsd")]
    todo!();

    #[cfg(target_os = "linux")]
    {
        let sz = rustix::fs::ioctl_blkpbszget(file)?;
        Ok(sz.into())
    }

    #[cfg(target_os = "macos")]
    {
        let sz = macos::ioctl_dkiocgetphysicalblocksize(file)?;
        Ok(sz.into())
    }

    #[cfg(target_os = "windows")]
    todo!();
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "windows"))]
pub fn minimum_io_size(file: &mut File) -> Result<u64, IoError> {
    #[cfg(target_os = "freebsd")]
    todo!();

    #[cfg(target_os = "linux")]
    {
        let sz = linux::ioctl_blkiomin(file)?;
        Ok(sz.into())
    }

    #[cfg(target_os = "macos")]
    todo!();

    #[cfg(target_os = "windows")]
    todo!();
}

#[cfg(target_os = "linux")]
pub fn optimal_io_size(file: &mut File) -> Result<u64, IoError> {
    let sz = linux::ioctl_blkioopt(file)?;
    Ok(sz.into())
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn alignment_offset(file: &mut File) -> Result<AlignmentOffset, IoError> {
    #[cfg(target_os = "freebsd")]
    todo!();

    #[cfg(target_os = "linux")]
    {
        let sz = linux::ioctl_blkalignoff(file)?;
        Ok(if sz >= 0 {
            AlignmentOffset::Offset(sz as u64)
        } else {
            AlignmentOffset::Misaligned
        })
    }

    #[cfg(target_os = "macos")]
    todo!();

    #[cfg(target_os = "windows")]
    todo!();
}
