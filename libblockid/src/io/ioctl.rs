#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

use crate::{
    error::Error,
    io::{File, block::Io},
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum AlignmentOffset {
    Misaligned,
    Offset(u64),
}

pub trait Ioctl: Io {
    fn device_size(&self) -> Result<u64, Error<Self::Error>>;

    fn logical_sector_size(&self) -> Result<u64, Error<Self::Error>>;

    fn physical_sector_size(&self) -> Result<u64, Error<Self::Error>>;

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn minimum_io_size(&self) -> Result<u64, Error<Self::Error>>;

    #[cfg(target_os = "linux")]
    fn optimal_io_size(&self) -> Result<u64, Error<Self::Error>>;

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn alignment_offset(&self) -> Result<crate::io::ioctl::AlignmentOffset, Error<Self::Error>>;
}

impl Ioctl for File {
    fn device_size(&self) -> Result<u64, Error<Self::Error>> {
        #[cfg(target_os = "freebsd")]
        todo!();

        #[cfg(target_os = "linux")]
        {
            let ds = crate::io::ioctl::linux::ioctl_blkgetsize64(self)?;
            return Ok(ds);
        }

        #[cfg(target_os = "macos")]
        todo!();
    }

    fn logical_sector_size(&self) -> Result<u64, Error<Self::Error>> {
        #[cfg(target_os = "freebsd")]
        todo!();

        #[cfg(target_os = "linux")]
        {
            let lssz = rustix::fs::ioctl_blksszget(self)?;
            Ok(lssz.into())
        }

        #[cfg(target_os = "macos")]
        {
            let lssz = macos::ioctl_dkiocgetblocksize(self)?;
            Ok(lssz.into())
        }
    }

    fn physical_sector_size(&self) -> Result<u64, Error<Self::Error>> {
        #[cfg(target_os = "freebsd")]
        todo!();

        #[cfg(target_os = "linux")]
        {
            let psz = rustix::fs::ioctl_blkpbszget(self)?;
            Ok(psz.into())
        }

        #[cfg(target_os = "macos")]
        {
            let psz = macos::ioctl_dkiocgetphysicalblocksize(self)?;
            Ok(psz.into())
        }
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn minimum_io_size(&self) -> Result<u64, Error<Self::Error>> {
        #[cfg(target_os = "freebsd")]
        todo!();

        #[cfg(target_os = "linux")]
        {
            let mios = linux::ioctl_blkiomin(self)?;
            Ok(mios.into())
        }

        #[cfg(target_os = "macos")]
        todo!();
    }

    #[cfg(target_os = "linux")]
    fn optimal_io_size(&self) -> Result<u64, Error<Self::Error>> {
        let oios = linux::ioctl_blkioopt(self)?;
        Ok(oios.into())
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn alignment_offset(&self) -> Result<crate::io::ioctl::AlignmentOffset, Error<Self::Error>> {
        #[cfg(target_os = "freebsd")]
        todo!();

        #[cfg(target_os = "linux")]
        {
            let alnoff = linux::ioctl_blkalignoff(self)?;
            Ok(if alnoff >= 0 {
                AlignmentOffset::Offset(alnoff as u64)
            } else {
                AlignmentOffset::Misaligned
            })
        }

        #[cfg(target_os = "macos")]
        todo!();
    }
}
