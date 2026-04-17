use libblockid_core::{BlockFilter, BlockInfo, LowProbe};

use crate::{error::Error, io::File};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum AlignmentOffset {
    Misaligned,
    Offset(u64),
}

#[derive(Debug)]
pub struct TopologyInfo {
    logical_sector_size: u64,
    physical_sector_size: u64,
    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "windows"))]
    minimum_io_size: u64,
    #[cfg(target_os = "linux")]
    optimal_io_size: u64,
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    alignment_offset: AlignmentOffset,
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

pub struct Probe {
    disk: File,
}

impl Probe {
    #[cfg(feature = "std")]
    pub fn new(file: File) -> Result<Probe, Error> {
        Ok(Self { disk: file })
    }

    #[cfg(all(feature = "no_std", target_family = "unix"))]
    pub fn new(fd: rustix::fd::OwnedFd) -> Result<Probe, Error> {
        Ok(Self { disk: fd.into() })
    }

    pub fn probe_block(&mut self, offset: u64, filter: BlockFilter) -> Result<BlockInfo, Error> {
        let mut low_probe = LowProbe::new(&mut self.disk, offset);

        let info = low_probe.probe_block(filter)?;

        Ok(info)
    }

    pub fn probe_topology(&mut self) -> Result<TopologyInfo, Error> {
        let logical_sector_size = crate::ioctl::logical_sector_size(&mut self.disk)?;
        let physical_sector_size = crate::ioctl::physical_sector_size(&mut self.disk)?;
        #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "windows"))]
        let minimum_io_size = crate::ioctl::minimum_io_size(&mut self.disk)?;
        #[cfg(target_os = "linux")]
        let optimal_io_size = crate::ioctl::optimal_io_size(&mut self.disk)?;
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        let alignment_offset = crate::ioctl::alignment_offset(&mut self.disk)?;

        Ok(TopologyInfo {
            logical_sector_size,
            physical_sector_size,
            #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "windows"))]
            minimum_io_size,
            #[cfg(target_os = "linux")]
            optimal_io_size,
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            alignment_offset,
        })
    }
}
