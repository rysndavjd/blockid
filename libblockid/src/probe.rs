use bstr::{BStr, BString, ByteSlice};
use widestring::{U16Str, U16String};

use crate::{
    error::Error,
    filesystem::{FS_DETECT_ORDER, FsFilter, FsInfo, FsType},
    io::{BlockIo, Reader},
    partition::{PT_DETECT_ORDER, PtFilter, PtInfo, PtType},
    std::fmt,
};

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

/// A label as raw bytes read from disk, tagged as UTF-8 or UTF-16.
///
/// The tag reflects what the superblock declares, not a guarantee that the
/// bytes are validly encoded. Callers may validate/convert the data to a
/// proper encoded string or use the raw bytes directly.
#[derive(Debug, Clone)]
pub enum Label {
    Utf8(BString),
    Utf16(U16String),
}

impl Label {
    /// Returns `true` if the label is [`Label::Utf8`].
    pub fn is_utf8(&self) -> bool {
        matches!(self, Label::Utf8(_))
    }

    /// Returns `true` if the label is [`Label::Utf16`].
    pub fn is_utf16(&self) -> bool {
        matches!(self, Label::Utf16(_))
    }

    /// Returns the label as a [`BStr`] if it is [`Label::Utf8`], otherwise `None`.
    pub fn as_bstr(&self) -> Option<&BStr> {
        match self {
            Label::Utf8(utf8) => Some(utf8.as_bstr()),
            _ => None,
        }
    }

    /// Returns the label as a [`U16Str`] if it is [`Label::Utf16`], otherwise `None`.
    pub fn as_u16str(&self) -> Option<&U16Str> {
        match self {
            Label::Utf16(utf16) => Some(utf16.as_ustr()),
            _ => None,
        }
    }

    /// Consumes the label and returns the inner [`BString`] if it is
    /// [`Label::Utf8`], otherwise `None`.
    pub fn into_bstring(self) -> Option<BString> {
        match self {
            Label::Utf8(utf8) => Some(utf8),
            _ => None,
        }
    }

    /// Consumes the label and returns the inner [`U16String`] if it is
    /// [`Label::Utf16`], otherwise `None`.
    pub fn into_u16string(self) -> Option<U16String> {
        match self {
            Label::Utf16(utf16) => Some(utf16),
            _ => None,
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Label {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Label::Utf8(bs) => serializer.serialize_str(bs.to_str_lossy().trim_end_matches('\0')),
            Label::Utf16(utf16) => {
                serializer.serialize_str(utf16.to_string_lossy().trim_end_matches('\0'))
            }
        }
    }
}

impl From<BString> for Label {
    fn from(v: BString) -> Self {
        Label::Utf8(v)
    }
}

impl From<U16String> for Label {
    fn from(v: U16String) -> Self {
        Label::Utf16(v)
    }
}

impl fmt::Display for Label {
    /// Formats the label as a lossily converted UTF-8 string.
    ///
    /// For both [`Label::Utf8`] and [`Label::Utf16`], invalid sequences are
    /// replaced with `U+FFFD`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Label::Utf8(utf8) => write!(f, "{}", utf8),
            Label::Utf16(utf16) => write!(f, "{}", utf16.to_string_lossy()),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Magic {
    /// Magic byte slice.
    pub bytes: &'static [u8],
    /// Offset where magic is found.
    pub offset: u64,
}

impl Magic {
    const EMPTY_MAGIC: Magic = Magic {
        bytes: &[0],
        offset: 0,
    };
}

fn probe_filesystem<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    filter: FsFilter,
) -> Result<FsInfo, Error<IO::Error>> {
    for block in FS_DETECT_ORDER {
        if filter.contains(block.0) {
            continue;
        }

        let handle = block.1.fs_handler();

        #[cfg(feature = "os_calls")]
        {
            if reader.os_calls() {
                if let Some(minsz) = handle.minsz
                    && reader.device_size()? < minsz
                {
                    return Err(Error::DeviceTooSmall);
                }
            } else {
                if let Some(minsz) = handle.minsz
                    && reader.seek(crate::io::SeekFrom::End(0))? < minsz
                {
                    return Err(Error::DeviceTooSmall);
                }
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

        match (handle.probe)(reader, offset, magic) {
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

fn search_for_filesystem<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    filesystem: FsType,
) -> Result<FsInfo, Error<IO::Error>> {
    let handle = filesystem.fs_handler::<IO>();

    #[cfg(feature = "os_calls")]
    {
        if reader.os_calls() {
            if let Some(minsz) = handle.minsz
                && reader.device_size()? < minsz
            {
                return Err(Error::DeviceTooSmall);
            }
        } else {
            if let Some(minsz) = handle.minsz
                && reader.seek(crate::io::SeekFrom::End(0))? < minsz
            {
                return Err(Error::DeviceTooSmall);
            }
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

    (handle.probe)(reader, offset, magic)
}

fn probe_part_table<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    filter: PtFilter,
) -> Result<PtInfo, Error<IO::Error>> {
    for block in PT_DETECT_ORDER {
        if filter.contains(block.0) {
            continue;
        }

        let handle = block.1.pt_handler();

        #[cfg(feature = "os_calls")]
        {
            if reader.os_calls() {
                if let Some(minsz) = handle.minsz
                    && reader.device_size()? < minsz
                {
                    return Err(Error::DeviceTooSmall);
                }
            } else {
                if let Some(minsz) = handle.minsz
                    && reader.seek(crate::io::SeekFrom::End(0))? < minsz
                {
                    return Err(Error::DeviceTooSmall);
                }
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

        match (handle.probe)(reader, offset, magic) {
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
    offset: u64,
    part_table: PtType,
) -> Result<PtInfo, Error<IO::Error>> {
    let handle = part_table.pt_handler::<IO>();

    #[cfg(feature = "os_calls")]
    {
        if reader.os_calls() {
            if let Some(minsz) = handle.minsz
                && reader.device_size()? < minsz
            {
                return Err(Error::DeviceTooSmall);
            }
        } else {
            if let Some(minsz) = handle.minsz
                && reader.seek(crate::io::SeekFrom::End(0))? < minsz
            {
                return Err(Error::DeviceTooSmall);
            }
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

    (handle.probe)(reader, offset, magic)
}

/// Probe for detecting filesystems and partition tables on a block device.
#[derive(Debug)]
pub struct Probe<IO: BlockIo> {
    /// Underlying `IO` source to probe.
    reader: Reader<IO>,
    /// Byte offset into the device at which probing begins.
    offset: u64,
}

#[cfg(not(feature = "os_calls"))]
impl<IO: BlockIo> Probe<IO> {
    /// Creates a new [`Probe`] for the given block device reader.
    ///
    /// # Parameters
    ///
    /// - `reader`: The underlying `IO` source to probe.
    /// - `offset`: Byte offset into the device at which probing begins.
    ///
    pub fn new(reader: IO, offset: u64) -> Result<Probe<IO>, Error<IO::Error>> {
        let mut io = Reader::new(reader);

        if offset >= io.seek(crate::io::SeekFrom::End(0))? {
            return Err(Error::OffsetExceedsDeviceSize);
        }

        Ok(Probe { reader: io, offset })
    }

    /// Probes the device for a filesystem, skipping any types set in `filter`.
    #[inline]
    pub fn probe_filesystem(&mut self, filter: FsFilter) -> Result<FsInfo, Error<IO::Error>> {
        probe_filesystem(&mut self.reader, self.offset, filter)
    }

    /// Probes the device for a specific filesystem.
    #[inline]
    pub fn search_for_filesystem(
        &mut self,
        filesystem: FsType,
    ) -> Result<FsInfo, Error<IO::Error>> {
        search_for_filesystem(&mut self.reader, self.offset, filesystem)
    }

    /// Probes the device for a partition table, skipping any types set in `filter`.
    #[inline]
    pub fn probe_part_table(&mut self, filter: PtFilter) -> Result<PtInfo, Error<IO::Error>> {
        probe_part_table(&mut self.reader, self.offset, filter)
    }

    /// Probes the device for a specific partition table.
    #[inline]
    pub fn search_for_part_table(
        &mut self,
        part_table: PtType,
    ) -> Result<PtInfo, Error<IO::Error>> {
        search_for_part_table(&mut self.reader, self.offset, part_table)
    }
}

#[cfg(feature = "os_calls")]
impl Probe<crate::io::File> {
    /// Creates a new [`Probe`] from an already open [`File`](crate::io::File).
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub fn from_file(
        file: crate::io::File,
        offset: u64,
    ) -> Result<Probe<crate::io::File>, Error<crate::io::IoError>> {
        use rustix::fs::{FileType, fstat};

        let os_calls = FileType::from_raw_mode(fstat(&file)?.st_mode).is_block_device();
        let reader = Reader::new(file, os_calls);

        if os_calls && offset >= reader.device_size()? {
            return Err(Error::OffsetExceedsDeviceSize);
        }

        Ok(Self { reader, offset })
    }

    /// Opens the block device at [`path`](std::path::Path) and creates a new [`Probe`] for it.
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub fn open<P: AsRef<std::path::Path>>(
        path: P,
        offset: u64,
    ) -> Result<Probe<crate::io::File>, Error<crate::io::IoError>> {
        let file = std::fs::File::open(path)?;

        Self::from_file(file, offset)
    }

    /// Creates a new [`Probe`] from an already open raw file descriptor `fd`.
    #[cfg(feature = "no_std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "no_std")))]
    pub fn from_fd(
        fd: rustix::fd::OwnedFd,
        offset: u64,
    ) -> Result<Probe<crate::io::File>, Error<crate::io::IoError>> {
        use rustix::fs::{FileType, fstat};

        let os_calls = FileType::from_raw_mode(fstat(&fd)?.st_mode).is_block_device();
        let reader = Reader::new(fd.into(), os_calls);

        if offset >= reader.device_size()? {
            return Err(Error::OffsetExceedsDeviceSize);
        }

        Ok(Self { reader, offset })
    }

    /// Opens the block device at `path` and creates a new [`Probe`] for it.
    #[cfg(feature = "no_std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "no_std")))]
    pub fn open<P: rustix::path::Arg>(
        path: P,
        offset: u64,
    ) -> Result<Probe<crate::io::File>, Error<crate::io::IoError>> {
        let fd = rustix::fs::open(path, rustix::fs::OFlags::RDONLY, rustix::fs::Mode::empty())?;

        Self::from_fd(fd, offset)
    }

    /// Probes the device for a filesystem, skipping any types set in `filter`.
    #[inline]
    pub fn probe_filesystem(
        &mut self,
        filter: FsFilter,
    ) -> Result<FsInfo, Error<crate::io::IoError>> {
        probe_filesystem(&mut self.reader, self.offset, filter)
    }

    /// Probes the device for a specific filesystem.
    #[inline]
    pub fn search_for_filesystem(
        &mut self,
        filesystem: FsType,
    ) -> Result<FsInfo, Error<crate::io::IoError>> {
        search_for_filesystem(&mut self.reader, self.offset, filesystem)
    }

    /// Probes the device for a partition table, skipping any types set in `filter`.
    #[inline]
    pub fn probe_part_table(
        &mut self,
        filter: PtFilter,
    ) -> Result<PtInfo, Error<crate::io::IoError>> {
        probe_part_table(&mut self.reader, self.offset, filter)
    }

    /// Probes the device for a specific partition table.
    #[inline]
    pub fn search_for_part_table(
        &mut self,
        part_table: PtType,
    ) -> Result<PtInfo, Error<crate::io::IoError>> {
        search_for_part_table(&mut self.reader, self.offset, part_table)
    }

    /// Returns the total size of the device, in bytes.
    #[inline]
    pub fn device_size(&self) -> Result<u64, Error<crate::io::IoError>> {
        if self.reader.os_calls() {
            self.reader.device_size()
        } else {
            Err(Error::ProbeNotBlockDevice)
        }
    }

    /// Returns the device's logical sector size, in bytes.
    #[inline]
    pub fn logical_sector_size(&self) -> Result<u64, Error<crate::io::IoError>> {
        if self.reader.os_calls() {
            self.reader.logical_sector_size()
        } else {
            Err(Error::ProbeNotBlockDevice)
        }
    }

    /// Returns the device's physical sector size, in bytes.
    #[inline]
    pub fn physical_sector_size(&self) -> Result<u64, Error<crate::io::IoError>> {
        if self.reader.os_calls() {
            self.reader.physical_sector_size()
        } else {
            Err(Error::ProbeNotBlockDevice)
        }
    }

    /// Returns the device's minimum I/O size, in bytes.
    #[inline]
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    #[cfg_attr(docsrs, doc(cfg(any(target_os = "linux", target_os = "freebsd"))))]
    pub fn minimum_io_size(&self) -> Result<u64, Error<crate::io::IoError>> {
        if self.reader.os_calls() {
            self.reader.minimum_io_size()
        } else {
            Err(Error::ProbeNotBlockDevice)
        }
    }

    /// Returns the device's optimal I/O size, in bytes.
    #[inline]
    #[cfg(target_os = "linux")]
    #[cfg_attr(docsrs, doc(cfg(target_os = "linux")))]
    pub fn optimal_io_size(&self) -> Result<u64, Error<crate::io::IoError>> {
        if self.reader.os_calls() {
            self.reader.optimal_io_size()
        } else {
            Err(Error::ProbeNotBlockDevice)
        }
    }

    /// Returns the device's alignment offset.
    #[inline]
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    #[cfg_attr(docsrs, doc(cfg(any(target_os = "linux", target_os = "freebsd"))))]
    pub fn alignment_offset(
        &self,
    ) -> Result<crate::io::ioctl::AlignmentOffset, Error<crate::io::IoError>> {
        if self.reader.os_calls() {
            self.reader.alignment_offset()
        } else {
            Err(Error::ProbeNotBlockDevice)
        }
    }
}
