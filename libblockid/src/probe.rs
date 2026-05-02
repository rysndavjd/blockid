use fat_volume_id::{VolumeId32, VolumeId64};
use uuid::Uuid;

use crate::{
    error::Error,
    filesystem::{BLOCK_DETECT_ORDER, BlockFilter, BlockInfo},
    io::{BlockIo, Reader},
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

#[cfg(all(not(feature = "os_calls"), feature = "no_std"))]
#[derive(Debug)]
pub struct Probe<IO: BlockIo> {
    reader: Reader<IO>,
    offset: u64,
}

#[cfg(all(not(feature = "os_calls"), feature = "no_std"))]
impl<IO: BlockIo> Probe<IO> {
    pub fn new(reader: IO, offset: u64) -> Probe<IO> {
        Probe {
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

#[cfg(feature = "os_calls")]
#[derive(Debug)]
pub struct Probe {
    disk: crate::io::File,
    offset: u64,
}

#[cfg(feature = "os_calls")]
impl Probe {
    #[cfg(feature = "std")]
    pub fn new(file: std::fs::File, offset: u64) -> Result<Probe, Error<std::io::Error>> {
        Ok(Self { disk: file, offset })
    }

    #[cfg(all(feature = "no_std", target_family = "unix"))]
    pub fn new(fd: rustix::fd::OwnedFd, offset: u64) -> Result<Probe, Error<crate::io::IoError>> {
        Ok(Self {
            disk: fd.into(),
            offset,
        })
    }
}
