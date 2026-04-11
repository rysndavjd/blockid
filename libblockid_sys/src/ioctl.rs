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

use crate::{error::Error, io::File, probe::AlignmentOffset};

pub fn ioctl_logical_sector_size(file: &mut File) -> Result<u64, Error> {
    // #[cfg(target_os = "freebsd")]
    #[cfg(target_os = "linux")]
    {
        let sz = rustix::fs::ioctl_blksszget(file)?;
        Ok(sz.into())
    }
    // #[cfg(target_os = "macos")]
    // #[cfg(target_os = "windows")]
}

pub fn ioctl_physical_sector_size(file: &mut File) -> Result<u64, Error> {
    // #[cfg(target_os = "freebsd")]
    #[cfg(target_os = "linux")]
    {
        let sz = rustix::fs::ioctl_blkpbszget(file)?;
        Ok(sz.into())
    }
    // #[cfg(target_os = "macos")]
    // #[cfg(target_os = "windows")]
}

pub fn ioctl_minimum_io_size(file: &mut File) -> Result<u64, Error> {
    // #[cfg(target_os = "freebsd")]
    #[cfg(target_os = "linux")]
    {
        let sz = linux::ioctl_blkiomin(file)?;
        Ok(sz.into())
    }
    // #[cfg(target_os = "macos")]
    // #[cfg(target_os = "windows")]
}

pub fn ioctl_optimal_io_size(file: &mut File) -> Result<u64, Error> {
    // #[cfg(target_os = "freebsd")]
    #[cfg(target_os = "linux")]
    {
        let sz = linux::ioctl_blkioopt(file)?;
        Ok(sz.into())
    }
    // #[cfg(target_os = "macos")]
    // #[cfg(target_os = "windows")]
}

pub fn ioctl_alignment_offset(file: &mut File) -> Result<AlignmentOffset, Error> {
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
