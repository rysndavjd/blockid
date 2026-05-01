use fat_volume_id::{VolumeId32, VolumeId64};
use uuid::Uuid;

use crate::{
    error::Error,
    filesystem::{BLOCK_DETECT_ORDER, BlockFilter, BlockInfo},
    io::{BlockIo, File, IoError, Reader},
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Usage {
    Filesystem,
    PartitionTable,
    Raid,
    Crypto,
    Other(&'static str),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Id {
    Uuid(Uuid),
    VolumeId32(VolumeId32),
    VolumeId64(VolumeId64),
}

impl Id {
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            Id::Uuid(t) => Some(*t),
            _ => None,
        }
    }

    pub fn as_volumeid32(&self) -> Option<VolumeId32> {
        match self {
            Id::VolumeId32(t) => Some(*t),
            _ => None,
        }
    }

    pub fn as_volumeid64(&self) -> Option<VolumeId64> {
        match self {
            Id::VolumeId64(t) => Some(*t),
            _ => None,
        }
    }
}

impl From<Uuid> for Id {
    fn from(value: Uuid) -> Self {
        Id::Uuid(value)
    }
}

impl From<VolumeId32> for Id {
    fn from(value: VolumeId32) -> Self {
        Id::VolumeId32(value)
    }
}

impl From<VolumeId64> for Id {
    fn from(value: VolumeId64) -> Self {
        Id::VolumeId64(value)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Endianness {
    Little,
    Big,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Magic {
    pub magic: &'static [u8],
    pub len: usize,
    pub b_offset: u64,
}

impl Magic {
    pub const EMPTY_MAGIC: Magic = Magic {
        magic: &[0],
        len: 0,
        b_offset: 0,
    };

    // fn is_empty(&self) -> bool {
    //     self == &Magic::EMPTY_MAGIC
    // }
}

#[derive(Debug)]
pub struct LowProbe<IO: BlockIo> {
    reader: Reader<IO>,
    offset: u64,
}

impl<IO: BlockIo> LowProbe<IO> {
    pub fn new(reader: IO, offset: u64) -> LowProbe<IO> {
        LowProbe {
            reader: Reader::new(reader),
            offset,
        }
    }

    pub fn probe_block(
        &mut self,
        block_filter: BlockFilter,
    ) -> Result<BlockInfo, Error<IO::Error>> {
        for block in BLOCK_DETECT_ORDER {
            if block_filter.contains(block.0) {
                continue;
            }

            let handle = block.1.block_handler::<IO>();

            let magic = match handle.magics {
                Some(magics) => match self.reader.get_magic(magics)? {
                    Some(magic) => magic,
                    None => continue,
                },
                None => Magic::EMPTY_MAGIC,
            };

            match (handle.probe)(&mut self.reader, self.offset, magic) {
                Ok(t) => return Ok(t),
                Err(e) => {
                    if let Error::Io(_) = e {
                        return Err(e);
                    }
                }
            };
        }
        return Err(Error::ProbesExhausted);
    }
}

pub struct Probe {
    disk: File,
}

#[cfg(feature = "os_calls")]
impl Probe {
    #[cfg(feature = "std")]
    pub fn new(file: File) -> Result<Probe, Error> {
        Ok(Self { disk: file })
    }

    #[cfg(all(feature = "no_std", target_family = "unix"))]
    pub fn new(fd: rustix::fd::OwnedFd) -> Result<Probe, Error<IoError>> {
        Ok(Self { disk: fd.into() })
    }

    pub fn probe_block(
        &mut self,
        offset: u64,
        block_filter: BlockFilter,
    ) -> Result<BlockInfo, Error<IoError>> {
        let mut low_probe = LowProbe::new(&mut self.disk, offset);

        let info = low_probe.probe_block(block_filter)?;

        Ok(info)
    }

    pub fn probe_topology(&mut self) -> Result<crate::topology::TopologyInfo, Error<IoError>> {
        let logical_sector_size =
            crate::topology::logical_sector_size(&mut self.disk).map_err(Error::Io)?;
        let physical_sector_size = crate::topology::physical_sector_size(&mut self.disk).map_err(Error::Io)?;
        #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "windows"))]
        let minimum_io_size = crate::topology::minimum_io_size(&mut self.disk).map_err(Error::Io)?;
        #[cfg(target_os = "linux")]
        let optimal_io_size = crate::topology::optimal_io_size(&mut self.disk).map_err(Error::Io)?;
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        let alignment_offset = crate::topology::alignment_offset(&mut self.disk).map_err(Error::Io)?;

        Ok(crate::topology::TopologyInfo {
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
