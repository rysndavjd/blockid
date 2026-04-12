#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

/* Note:
 * The rustix::ioctl::opcode::read function seems to calculate different values
 * on different systems. For example:
 *
 *   read::<u32>(b'd', 24) == 2147771416 on Linux
 *   read::<u32>(b'd', 24) == 1074029592 on macOS
 */

use crate::{error::Error, io::File};

pub fn logical_sector_size(file: &mut File) -> Result<u64, Error> {
    // #[cfg(target_os = "freebsd")]
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
    // #[cfg(target_os = "windows")]
}

pub fn physical_sector_size(file: &mut File) -> Result<u64, Error> {
    // #[cfg(target_os = "freebsd")]
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
    // #[cfg(target_os = "windows")]
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "windows"))]
pub fn minimum_io_size(file: &mut File) -> Result<u64, Error> {
    // #[cfg(target_os = "freebsd")]
    #[cfg(target_os = "linux")]
    {
        let sz = linux::ioctl_blkiomin(file)?;
        Ok(sz.into())
    }
    // #[cfg(target_os = "macos")]
    // #[cfg(target_os = "windows")]
}

#[cfg(target_os = "linux")]
pub fn optimal_io_size(file: &mut File) -> Result<u64, Error> {
    // #[cfg(target_os = "freebsd")]
    #[cfg(target_os = "linux")]
    {
        let sz = linux::ioctl_blkioopt(file)?;
        Ok(sz.into())
    }
    // #[cfg(target_os = "macos")]
    // #[cfg(target_os = "windows")]
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn alignment_offset(file: &mut File) -> Result<AlignmentOffset, Error> {
    use crate::probe::AlignmentOffset;
    // #[cfg(target_os = "freebsd")]
    #[cfg(target_os = "linux")]
    {
        let sz = linux::ioctl_blkalignoff(file)?;
        Ok(if sz >= 0 {
            AlignmentOffset::Offset(sz as u64)
        } else {
            AlignmentOffset::Misaligned
        })
    }
    // #[cfg(target_os = "macos")]
    // #[cfg(target_os = "windows")]
}
