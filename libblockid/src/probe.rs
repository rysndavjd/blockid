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

/// Probe for detecting filesystems and partition tables on a block device.
///
/// # Type Parameters
///
/// - `IO`: A concrete type that implements:
///   - [`Read`](https://doc.rust-lang.org/std/io/trait.Read.html) +
///     [`Seek`](https://doc.rust-lang.org/std/io/trait.Seek.html) from the
///     [standard library](https://doc.rust-lang.org/std/index.html), or
///   - [`Read`](https://docs.rs/embedded-io/latest/embedded_io/trait.Read.html) +
///     [`Seek`](https://docs.rs/embedded-io/latest/embedded_io/trait.Seek.html) from
///     [embedded-io](https://docs.rs/embedded-io/latest/embedded_io/index.html),
///     depending on which feature is enabled.
#[derive(Debug)]
pub struct RawProbe<IO: BlockIo> {
    reader: Reader<IO>,
    flags: ProbeFlags,
    offset: u64,
}

impl<IO: BlockIo> RawProbe<IO> {
    /// Creates a new [`RawProbe`] for the given block device reader.
    ///
    /// # Parameters
    ///
    /// - `reader`: The underlying `IO` source to probe.
    /// - `flags`: Changes the behavior of the probe. See [`ProbeFlags`].
    /// - `offset`: Byte offset into the device at which probing begins.
    ///
    pub fn new(reader: IO, flags: ProbeFlags, offset: u64) -> RawProbe<IO> {
        RawProbe {
            reader: Reader::new(reader),
            flags,
            offset,
        }
    }

    pub fn probe_block(&mut self, filter: BlockFilter) -> Result<BlockInfo, Error<IO::Error>> {
        for block in BLOCK_DETECT_ORDER {
            if filter.contains(block.0) {
                continue;
            }

            let handle = block.1.block_handler();

            #[cfg(feature = "os_calls")]
            {
                if let Some(minsz) = handle.minsz
                    && self.reader.device_size()? < minsz
                {
                    continue;
                }
            }

            #[cfg(not(feature = "os_calls"))]
            {
                if let Some(minsz) = handle.minsz
                    && self.reader.seek(crate::io::SeekFrom::End(0))? < minsz
                {
                    continue;
                }
            }

            let magic = match handle.magics {
                Some(magics) => match self.reader.get_magic(magics)? {
                    Some(magic) => magic,
                    None => continue,
                },
                None => Magic::EMPTY_MAGIC,
            };

            match (handle.probe)(&mut self.reader, self.flags, self.offset, magic) {
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

    pub fn search_for_block(&mut self, block: BlockType) -> Result<BlockInfo, Error<IO::Error>> {
        let handle = block.block_handler::<IO>();

        #[cfg(feature = "os_calls")]
        {
            if let Some(minsz) = handle.minsz
                && self.reader.device_size()? < minsz
            {
                return Err(Error::DeviceTooSmall);
            }
        }

        #[cfg(not(feature = "os_calls"))]
        {
            if let Some(minsz) = handle.minsz
                && self.reader.seek(crate::io::SeekFrom::End(0))? < minsz
            {
                return Err(Error::DeviceTooSmall);
            }
        }

        let magic = match handle.magics {
            Some(magics) => match self.reader.get_magic(magics)? {
                Some(magic) => magic,
                None => return Err(Error::UnableToLocateMagicSignature),
            },
            None => Magic::EMPTY_MAGIC,
        };

        (handle.probe)(&mut self.reader, self.flags, self.offset, magic)
    }

    pub fn probe_part_table(
        &mut self,
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
                    && self.reader.device_size()? < minsz
                {
                    continue;
                }
            }

            #[cfg(not(feature = "os_calls"))]
            {
                if let Some(minsz) = handle.minsz
                    && self.reader.seek(crate::io::SeekFrom::End(0))? < minsz
                {
                    continue;
                }
            }

            let magic = match handle.magics {
                Some(magics) => match self.reader.get_magic(magics)? {
                    Some(magic) => magic,
                    None => continue,
                },
                None => Magic::EMPTY_MAGIC,
            };

            match (handle.probe)(&mut self.reader, self.flags, self.offset, magic) {
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

    pub fn search_for_part_table(
        &mut self,
        part_table: PTType,
    ) -> Result<PartTableInfo, Error<IO::Error>> {
        let handle = part_table.pt_handler::<IO>();

        #[cfg(feature = "os_calls")]
        {
            if let Some(minsz) = handle.minsz
                && self.reader.device_size()? < minsz
            {
                return Err(Error::DeviceTooSmall);
            }
        }

        #[cfg(not(feature = "os_calls"))]
        {
            if let Some(minsz) = handle.minsz
                && self.reader.seek(crate::io::SeekFrom::End(0))? < minsz
            {
                return Err(Error::DeviceTooSmall);
            }
        }

        let magic = match handle.magics {
            Some(magics) => match self.reader.get_magic(magics)? {
                Some(magic) => magic,
                None => return Err(Error::UnableToLocateMagicSignature),
            },
            None => Magic::EMPTY_MAGIC,
        };

        (handle.probe)(&mut self.reader, self.flags, self.offset, magic)
    }
}

#[cfg(feature = "os_calls")]
impl RawProbe<crate::io::File> {
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub fn from_file(
        file: crate::io::File,
        flags: ProbeFlags,
        offset: u64,
    ) -> Result<RawProbe<crate::io::File>, Error<crate::io::IoError>> {
        Ok(Self {
            reader: Reader::new(file),
            flags,
            offset,
        })
    }

    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub fn open<P: AsRef<std::path::Path>>(
        path: P,
        flags: ProbeFlags,
        offset: u64,
    ) -> Result<RawProbe<crate::io::File>, Error<crate::io::IoError>> {
        let file = std::fs::File::open(path)?;

        return Ok(Self {
            reader: Reader::new(file),
            flags,
            offset,
        });
    }

    #[cfg(feature = "no_std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "no_std")))]
    pub fn from_fd(
        fd: rustix::fd::OwnedFd,
        flags: ProbeFlags,
        offset: u64,
    ) -> Result<RawProbe<crate::io::File>, Error<crate::io::IoError>> {
        Ok(Self {
            reader: Reader::new(crate::io::File::from(fd)),
            flags,
            offset,
        })
    }

    #[cfg(feature = "no_std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "no_std")))]
    pub fn open<P: rustix::path::Arg>(
        path: P,
        flags: ProbeFlags,
        offset: u64,
    ) -> Result<RawProbe<crate::io::File>, Error<crate::io::IoError>> {
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
    #[cfg_attr(docsrs, doc(cfg(any(target_os = "linux", target_os = "freebsd"))))]
    pub fn minimum_io_size(&self) -> Result<u64, Error<crate::io::IoError>> {
        self.reader.minimum_io_size()
    }

    #[inline]
    #[cfg(target_os = "linux")]
    #[cfg_attr(docsrs, doc(cfg(target_os = "linux")))]
    pub fn optimal_io_size(&self) -> Result<u64, Error<crate::io::IoError>> {
        self.reader.optimal_io_size()
    }

    #[inline]
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    #[cfg_attr(docsrs, doc(cfg(any(target_os = "linux", target_os = "freebsd"))))]
    pub fn alignment_offset(
        &self,
    ) -> Result<crate::io::ioctl::AlignmentOffset, Error<crate::io::IoError>> {
        self.reader.alignment_offset()
    }
}
