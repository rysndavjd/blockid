#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use crate::{error::Error, io::File, io::block::Io};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum AlignmentOffset {
    Misaligned,
    Offset(u64),
}

pub trait Ioctl: Io {
    fn logical_sector_size(&mut self) -> Result<u64, Error<Self::Error>>;

    fn physical_sector_size(&mut self) -> Result<u64, Error<Self::Error>>;

    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "windows"))]
    fn minimum_io_size(&mut self) -> Result<u64, Error<Self::Error>>;

    #[cfg(target_os = "linux")]
    fn optimal_io_size(&mut self) -> Result<u64, Error<Self::Error>>;

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn alignment_offset(&mut self)
    -> Result<crate::io::ioctl::AlignmentOffset, Error<Self::Error>>;
}

impl Ioctl for File {
    fn logical_sector_size(&mut self) -> Result<u64, Error<Self::Error>> {
        #[cfg(target_os = "freebsd")]
        todo!();

        #[cfg(target_os = "linux")]
        {
            let sz = rustix::fs::ioctl_blksszget(self)?;
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

    fn physical_sector_size(&mut self) -> Result<u64, Error<Self::Error>> {
        #[cfg(target_os = "freebsd")]
        todo!();

        #[cfg(target_os = "linux")]
        {
            let sz = rustix::fs::ioctl_blkpbszget(self)?;
            Ok(sz.into())
        }

        #[cfg(target_os = "macos")]
        {
            let sz = macos::ioctl_dkiocgetphysicalblocksize(self)?;
            Ok(sz.into())
        }

        #[cfg(target_os = "windows")]
        todo!();
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "windows"))]
    fn minimum_io_size(&mut self) -> Result<u64, Error<Self::Error>> {
        #[cfg(target_os = "freebsd")]
        todo!();

        #[cfg(target_os = "linux")]
        {
            let sz = linux::ioctl_blkiomin(self)?;
            Ok(sz.into())
        }

        #[cfg(target_os = "macos")]
        todo!();

        #[cfg(target_os = "windows")]
        todo!();
    }

    #[cfg(target_os = "linux")]
    fn optimal_io_size(&mut self) -> Result<u64, Error<Self::Error>> {
        let sz = linux::ioctl_blkioopt(self)?;
        Ok(sz.into())
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn alignment_offset(
        &mut self,
    ) -> Result<crate::io::ioctl::AlignmentOffset, Error<Self::Error>> {
        #[cfg(target_os = "freebsd")]
        todo!();

        #[cfg(target_os = "linux")]
        {
            let sz = linux::ioctl_blkalignoff(self)?;
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
}
