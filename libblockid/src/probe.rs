use bitflags::bitflags;
use fat_volume_id::{VolumeId32, VolumeId64};
use uuid::Uuid;

use crate::{
    error::Error,
    filesystem::{BLOCK_DETECT_ORDER, BlockFilter, BlockInfo, BlockType},
    io::{BlockIo, Reader},
    partition::{PT_DETECT_ORDER, PTFilter, PTType, PartTableInfo},
};

/// Describes the intended usage of a superblock.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Usage {
    /// Stores files and directories in a structured filesystem.
    Filesystem,
    /// Spans or mirrors data across multiple physical disks (RAID).
    Raid,
    /// Manages an encrypted volume or backing store.
    Crypto,
    Other(&'static str),
}

/// Identifier used by a filesystem or partition table.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Id {
    /// A 128-bit universally unique identifier.
    Uuid(Uuid),
    /// A 32-bit MBR disk signature.
    Mbr { disk: u32 },
    /// A 32-bit volume serial number.
    VolumeId32(VolumeId32),
    /// A 64-bit volume serial number.
    VolumeId64(VolumeId64),
}

impl Id {
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            Id::Uuid(t) => Some(*t),
            _ => None,
        }
    }

    pub fn as_mbr(&self) -> Option<u32> {
        match self {
            Id::Mbr { disk } => Some(*disk),
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

/// The byte order used to represent multi-byte values.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Endianness {
    /// Least significant byte stored first.
    Little,
    /// Most significant byte stored first.
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
}

bitflags! {
    /// Flags that control the behaviour of the probing process.
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct ProbeFlags: u64 {
        /// Return an error if a UTF string encountered during probing is invalid.
        const FailOnInvaildUTF = 1 << 0;
    }
}

fn probe_block<IO: BlockIo>(
    reader: &mut Reader<IO>,
    flags: ProbeFlags,
    offset: u64,
    filter: BlockFilter,
) -> Result<BlockInfo, Error<IO::Error>> {
    for block in BLOCK_DETECT_ORDER {
        if filter.contains(block.0) {
            continue;
        }

        let handle = block.1.block_handler();

        #[cfg(feature = "os_calls")]
        {
            if let Some(minsz) = handle.minsz
                && reader.device_size()? < minsz
            {
                continue;
            }
        }

        let magic = match handle.magics {
            Some(magics) => match reader.get_magic(magics)? {
                Some(magic) => magic,
                None => continue,
            },
            None => Magic::EMPTY_MAGIC,
        };

        match (handle.probe)(reader, flags, offset, magic) {
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

fn search_for_block<IO: BlockIo>(
    reader: &mut Reader<IO>,
    flags: ProbeFlags,
    offset: u64,
    block: BlockType,
) -> Result<BlockInfo, Error<IO::Error>> {
    let handle = block.block_handler::<IO>();

    #[cfg(feature = "os_calls")]
    {
        if let Some(minsz) = handle.minsz
            && reader.device_size()? < minsz
        {
            return Err(Error::DeviceTooSmall);
        }
    }

    let magic = match handle.magics {
        Some(magics) => match reader.get_magic(magics)? {
            Some(magic) => magic,
            None => return Err(Error::UnableToLocateMagicSignature),
        },
        None => Magic::EMPTY_MAGIC,
    };

    (handle.probe)(reader, flags, offset, magic)
}

fn probe_part_table<IO: BlockIo>(
    reader: &mut Reader<IO>,
    flags: ProbeFlags,
    offset: u64,
    filter: PTFilter,
) -> Result<PartTableInfo, Error<IO::Error>> {
    for block in PT_DETECT_ORDER {
        if filter.contains(block.0) {
            continue;
        }

        let handle = block.1.pt_handler();

        #[cfg(feature = "os_calls")]
        {
            if let Some(minsz) = handle.minsz
                && reader.device_size()? < minsz
            {
                continue;
            }
        }

        let magic = match handle.magics {
            Some(magics) => match reader.get_magic(magics)? {
                Some(magic) => magic,
                None => continue,
            },
            None => Magic::EMPTY_MAGIC,
        };

        match (handle.probe)(reader, flags, offset, magic) {
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

fn search_for_part_table<IO: BlockIo>(
    reader: &mut Reader<IO>,
    flags: ProbeFlags,
    offset: u64,
    part_table: PTType,
) -> Result<PartTableInfo, Error<IO::Error>> {
    let handle = part_table.pt_handler::<IO>();

    #[cfg(feature = "os_calls")]
    {
        if let Some(minsz) = handle.minsz
            && reader.device_size()? < minsz
        {
            return Err(Error::DeviceTooSmall);
        }
    }

    let magic = match handle.magics {
        Some(magics) => match reader.get_magic(magics)? {
            Some(magic) => magic,
            None => return Err(Error::UnableToLocateMagicSignature),
        },
        None => Magic::EMPTY_MAGIC,
    };

    (handle.probe)(reader, flags, offset, magic)
}

#[cfg(not(feature = "os_calls"))]
#[doc(cfg(not(feature = "os_calls")))]
#[derive(Debug)]
pub struct Probe<IO: BlockIo> {
    reader: Reader<IO>,
    flags: ProbeFlags,
    offset: u64,
}

#[cfg(not(feature = "os_calls"))]
#[doc(cfg(not(feature = "os_calls")))]
impl<IO: BlockIo> Probe<IO> {
    pub fn new(reader: IO, flags: ProbeFlags, offset: u64) -> Probe<IO> {
        Probe {
            reader: Reader::new(reader),
            flags,
            offset,
        }
    }

    #[inline]
    pub fn probe_block(&mut self, filter: BlockFilter) -> Result<BlockInfo, Error<IO::Error>> {
        probe_block(&mut self.reader, self.flags, self.offset, filter)
    }

    #[inline]
    pub fn search_for_block(&mut self, block: BlockType) -> Result<BlockInfo, Error<IO::Error>> {
        search_for_block(&mut self.reader, self.flags, self.offset, block)
    }

    #[inline]
    pub fn probe_part_table(
        &mut self,
        filter: PTFilter,
    ) -> Result<PartTableInfo, Error<IO::Error>> {
        probe_part_table(&mut self.reader, self.flags, self.offset, filter)
    }

    #[inline]
    pub fn search_for_part_table(
        &mut self,
        part_table: PTType,
    ) -> Result<PartTableInfo, Error<IO::Error>> {
        search_for_part_table(&mut self.reader, self.flags, self.offset, part_table)
    }
}

#[cfg(feature = "os_calls")]
#[doc(cfg(feature = "os_calls"))]
#[derive(Debug)]
pub struct Probe {
    reader: Reader<crate::io::File>,
    flags: ProbeFlags,
    offset: u64,
}

#[cfg(feature = "os_calls")]
#[doc(cfg(feature = "os_calls"))]
impl Probe {
    #[cfg(feature = "std")]
    #[doc(cfg(feature = "std"))]
    pub fn new(
        file: crate::io::File,
        flags: ProbeFlags,
        offset: u64,
    ) -> Result<Probe, Error<crate::io::IoError>> {
        Ok(Self {
            reader: Reader::new(file),
            flags,
            offset,
        })
    }

    #[cfg(feature = "std")]
    #[doc(cfg(feature = "std"))]
    pub fn open<P: AsRef<std::path::Path>>(
        path: P,
        flags: ProbeFlags,
        offset: u64,
    ) -> Result<Probe, Error<crate::io::IoError>> {
        let file = std::fs::File::open(path)?;

        return Ok(Self {
            reader: Reader::new(file),
            flags,
            offset,
        });
    }

    #[cfg(feature = "no_std")]
    #[doc(cfg(feature = "no_std"))]
    pub fn new(
        fd: rustix::fd::OwnedFd,
        flags: ProbeFlags,
        offset: u64,
    ) -> Result<Probe, Error<crate::io::IoError>> {
        Ok(Self {
            reader: Reader::new(crate::io::File::from(fd)),
            flags,
            offset,
        })
    }

    #[cfg(feature = "no_std")]
    #[doc(cfg(feature = "no_std"))]
    pub fn open<P: rustix::path::Arg>(
        path: P,
        flags: ProbeFlags,
        offset: u64,
    ) -> Result<Probe, Error<crate::io::IoError>> {
        let fd = rustix::fs::open(path, rustix::fs::OFlags::RDONLY, rustix::fs::Mode::empty())?;

        return Ok(Self {
            reader: Reader::new(crate::io::File::from(fd)),
            flags,
            offset,
        });
    }

    #[inline]
    pub fn device_size(&self) -> Result<u64, Error<crate::io::IoError>> {
        self.reader.device_size()
    }

    #[inline]
    pub fn logical_sector_size(&self) -> Result<u64, Error<crate::io::IoError>> {
        self.reader.logical_sector_size()
    }

    #[inline]
    pub fn physical_sector_size(&self) -> Result<u64, Error<crate::io::IoError>> {
        self.reader.physical_sector_size()
    }

    #[inline]
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    #[doc(cfg(any(target_os = "linux", target_os = "freebsd")))]
    pub fn minimum_io_size(&self) -> Result<u64, Error<crate::io::IoError>> {
        self.reader.minimum_io_size()
    }

    #[inline]
    #[cfg(target_os = "linux")]
    #[doc(cfg(target_os = "linux"))]
    pub fn optimal_io_size(&self) -> Result<u64, Error<crate::io::IoError>> {
        self.reader.optimal_io_size()
    }

    #[inline]
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    #[doc(cfg(any(target_os = "linux", target_os = "freebsd")))]
    pub fn alignment_offset(
        &self,
    ) -> Result<crate::io::ioctl::AlignmentOffset, Error<crate::io::IoError>> {
        self.reader.alignment_offset()
    }

    #[inline]
    pub fn probe_block(
        &mut self,
        filter: BlockFilter,
    ) -> Result<BlockInfo, Error<crate::io::IoError>> {
        probe_block(&mut self.reader, self.flags, self.offset, filter)
    }

    #[inline]
    pub fn search_for_block(
        &mut self,
        block: BlockType,
    ) -> Result<BlockInfo, Error<crate::io::IoError>> {
        search_for_block(&mut self.reader, self.flags, self.offset, block)
    }

    #[inline]
    pub fn probe_part_table(
        &mut self,
        filter: PTFilter,
    ) -> Result<PartTableInfo, Error<crate::io::IoError>> {
        probe_part_table(&mut self.reader, self.flags, self.offset, filter)
    }

    #[inline]
    pub fn search_for_part_table(
        &mut self,
        part_table: PTType,
    ) -> Result<PartTableInfo, Error<crate::io::IoError>> {
        search_for_part_table(&mut self.reader, self.flags, self.offset, part_table)
    }
}
