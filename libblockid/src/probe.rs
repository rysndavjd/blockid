use bitflags::bitflags;

use crate::{
    error::Error,
    filesystem::{BLOCK_DETECT_ORDER, BlockFilter, BlockInfo, BlockType},
    io::{BlockIo, Reader},
    partition::{PT_DETECT_ORDER, PTFilter, PartTableInfo, PartTableType},
};

/// Describes the intended usage of a superblock.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Usage {
    /// Stores files and directories in a structured filesystem.
    Filesystem,
    /// Spans or mirrors data across multiple physical disks (RAID).
    Raid,
    /// Manages an encrypted volume or backing store.
    Crypto,
}

/// The byte order used to represent multi-byte values.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Endianness {
    /// Least significant byte stored first.
    Little,
    /// Most significant byte stored first.
    Big,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Magic {
    pub magic: &'static [u8],
    pub len: usize,
    pub b_offset: u64,
}

impl Magic {
    const EMPTY_MAGIC: Magic = Magic {
        magic: &[0],
        len: 0,
        b_offset: 0,
    };
}

bitflags! {
    /// Flags that control the behaviour of the probing process.
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct ProbeFlags: u64 {
        /// Return an error if a UTF string encountered during probing is Invalid.
        const FailOnInvalidUTF = 1 << 0;
    }
}

pub fn probe_block<IO: BlockIo>(
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

        #[cfg(not(feature = "os_calls"))]
        {
            if let Some(minsz) = handle.minsz
                && reader.seek(crate::io::SeekFrom::End(0))? < minsz
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

pub fn search_for_block<IO: BlockIo>(
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

    #[cfg(not(feature = "os_calls"))]
    {
        if let Some(minsz) = handle.minsz
            && reader.seek(crate::io::SeekFrom::End(0))? < minsz
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

pub fn probe_part_table<IO: BlockIo>(
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

        #[cfg(not(feature = "os_calls"))]
        {
            if let Some(minsz) = handle.minsz
                && reader.seek(crate::io::SeekFrom::End(0))? < minsz
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

pub fn search_for_part_table<IO: BlockIo>(
    reader: &mut Reader<IO>,
    flags: ProbeFlags,
    offset: u64,
    part_table: PartTableType,
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

    #[cfg(not(feature = "os_calls"))]
    {
        if let Some(minsz) = handle.minsz
            && reader.seek(crate::io::SeekFrom::End(0))? < minsz
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

/// Probe for detecting filesystems and partition tables on a block device.
#[derive(Debug)]
pub struct Probe<IO: BlockIo> {
    reader: Reader<IO>,
    flags: ProbeFlags,
    offset: u64,
}

#[cfg(not(feature = "os_calls"))]
impl<IO: BlockIo> Probe<IO> {
    /// Creates a new [`Probe`] for the given block device reader.
    ///
    /// # Parameters
    ///
    /// - `reader`: The underlying `IO` source to probe.
    /// - `flags`: Changes the behavior of the probe. See [`ProbeFlags`].
    /// - `offset`: Byte offset into the device at which probing begins.
    ///
    pub fn new(reader: IO, flags: ProbeFlags, offset: u64) -> Result<Probe<IO>, Error<IO::Error>> {
        let mut io = Reader::new(reader);

        if offset >= io.seek(crate::io::SeekFrom::End(0))? {
            return Err(Error::OffsetExceedsDeviceSize);
        }

        Ok(Probe {
            reader: io,
            flags,
            offset,
        })
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
impl Probe<crate::io::File> {
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub fn from_file(
        file: crate::io::File,
        flags: ProbeFlags,
        offset: u64,
    ) -> Result<Probe<crate::io::File>, Error<crate::io::IoError>> {
        let reader = Reader::new(file);

        if offset >= reader.device_size()? {
            return Err(Error::OffsetExceedsDeviceSize);
        }

        Ok(Self {
            reader,
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
    ) -> Result<Probe<crate::io::File>, Error<crate::io::IoError>> {
        let file = std::fs::File::open(path)?;

        Self::from_file(file, flags, offset)
    }

    #[cfg(feature = "no_std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "no_std")))]
    pub fn from_fd(
        fd: rustix::fd::OwnedFd,
        flags: ProbeFlags,
        offset: u64,
    ) -> Result<Probe<crate::io::File>, Error<crate::io::IoError>> {
        let reader = Reader::new(fd.into());

        if offset >= reader.device_size()? {
            return Err(Error::OffsetExceedsDeviceSize);
        }

        Ok(Self {
            reader,
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
    ) -> Result<Probe<crate::io::File>, Error<crate::io::IoError>> {
        let fd = rustix::fs::open(path, rustix::fs::OFlags::RDONLY, rustix::fs::Mode::empty())?;

        Self::from_fd(fd, flags, offset)
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
        part_table: PartTableType,
    ) -> Result<PartTableInfo, Error<crate::io::IoError>> {
        search_for_part_table(&mut self.reader, self.flags, self.offset, part_table)
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
