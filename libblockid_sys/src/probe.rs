use libblockid_core::{BlockFilter, BlockInfo, LowProbe};

use crate::{
    error::Error,
    io::File,
    ioctl::{
        ioctl_alignment_offset, ioctl_logical_sector_size, ioctl_minimum_io_size,
        ioctl_optimal_io_size, ioctl_physical_sector_size,
    },
};

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
    pub fn new(file: File) -> Result<Probe, Error> {
        Ok(Self { disk: file })
    }

    pub fn probe_info(&mut self, offset: u64, filter: BlockFilter) -> Result<BlockInfo, Error> {
        let mut low_probe = LowProbe::new(&mut self.disk, offset);

        let info = low_probe.probe(filter)?;

        Ok(info)
    }

    pub fn probe_topology(&mut self) -> Result<TopologyInfo, Error> {
        #[cfg(target_os = "linux")]
        {
            let logical_sector_size = ioctl_logical_sector_size(&mut self.disk)?;
            let physical_sector_size = ioctl_physical_sector_size(&mut self.disk)?;
            let minimum_io_size = ioctl_minimum_io_size(&mut self.disk)?;
            let optimal_io_size = ioctl_optimal_io_size(&mut self.disk)?;
            let alignment_offset = ioctl_alignment_offset(&mut self.disk)?;

            Ok(TopologyInfo {
                logical_sector_size,
                physical_sector_size,
                minimum_io_size,
                optimal_io_size,
                alignment_offset,
            })
        }
    }
}
