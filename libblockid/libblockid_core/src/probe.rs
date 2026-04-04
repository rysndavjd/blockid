use bitflags::bitflags;
use fat_volume_id::{VolumeId32, VolumeId64};
use uuid::Uuid;

use crate::{
    Read, Seek, SeekFrom,
    error::{Error, ErrorKind},
    filesystem::vfat::{VFAT_MAGICS, probe_vfat},
};

#[derive(Debug)]
pub struct Reader<R: Read + Seek>(R);

impl<R: Read + Seek> Reader<R> {
    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
        self.0.seek(pos).map_err(|_| ErrorKind::Todo.into())
    }

    pub fn rewind(&mut self) -> Result<(), Error> {
        self.0.rewind().map_err(|_| ErrorKind::Todo.into())
    }

    pub fn stream_position(&mut self) -> Result<u64, Error> {
        self.0.stream_position().map_err(|_| ErrorKind::Todo.into())
    }

    pub fn seek_relative(&mut self, offset: i64) -> Result<(), Error> {
        self.0
            .seek_relative(offset)
            .map_err(|_| ErrorKind::Todo.into())
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        self.0.read(buf).map_err(|_| ErrorKind::Todo.into())
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        self.0.read_exact(buf).map_err(|_| ErrorKind::Todo.into())
    }

    pub fn read_exact_at<const S: usize>(&mut self, offset: u64) -> Result<[u8; S], Error> {
        let mut buffer = [0u8; S];
        self.seek(SeekFrom::Start(offset))?;
        self.read_exact(&mut buffer)?;

        return Ok(buffer);
    }

    pub fn read_vec_at(&mut self, offset: u64, buf_size: usize) -> Result<Vec<u8>, Error> {
        let mut buffer = vec![0u8; buf_size];
        self.seek(SeekFrom::Start(offset))?;
        self.read_exact(&mut buffer)?;

        return Ok(buffer);
    }
}

#[derive(Debug, Copy, Clone, Hash)]
pub struct SuperblockInfo<R: Read + Seek> {
    pub usage: Usage,
    pub minsz: Option<u64>,
    pub magics: Option<&'static [Magic]>,
    pub probe: fn(&mut Reader<R>, u64, Magic) -> Result<BlockInfo, Error>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BlockType {
    LUKS1,
    LUKS2,
    LUKSOpal,
    Dos,
    Gpt,
    Exfat,
    Jbd,
    Apfs,
    Ext2,
    Ext3,
    Ext4,
    LinuxSwapV0,
    LinuxSwapV1,
    SwapSuspend,
    Ntfs,
    Vfat,
    Xfs,
    Squashfs,
    Squashfs3,
    ZoneFs,
    Lvm2Member,
    Lvm1Member,
    LvmSnapcow,
    LvmVerityHash,
    LvmIntegrity,
}

impl BlockType {
    fn block_info<R: Read + Seek>(&self) -> SuperblockInfo<R> {
        match self {
            BlockType::Vfat => SuperblockInfo {
                usage: Usage::Filesystem,
                minsz: None,
                magics: VFAT_MAGICS,
                probe: probe_vfat,
            },
            _ => todo!(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SecType {
    Fat12,
    Fat16,
    Fat32,
}

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
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tag {
    /// Filesystem type, Eg: EXT4.
    FsType(BlockType),
    /// Secondary filesystem type, Eg: Filsystem is VFAT but secondary type is FAT16.
    SecType(SecType),
    /// Filesystem label, Eg: `LABEL`.
    Label(String),
    /// Filesystem identifier in lower case hex.
    /// Eg:
    ///     UUID: `67e55044-10b1-426f-9247-bb680e5fe0c8`
    ///     VolumeId32: `2a9d-b913`
    ///     VolumeId64: `17acf19235bcde78`
    Id(Id),
    /// Sub member identifier in lower case hex.
    SubMemberId(Id),
    /// External log identifier in lower case hex.
    ExtLogId(Id),
    /// External journal identifier in lower case hex.
    ExtJournalId(Id),
    /// Usage string, Eg: `raid`, `filesystem`.
    Usage(Usage),
    /// Filesystem version.
    Version(String),
    /// Superblock magic string.
    Magic(Vec<u8>),
    /// Superblock magic string offset.
    MagicOffset(u64),
    /// Filesystem size.
    FsSize(u64),
    /// Last fsblock/total number of fsblocks.
    FsLastBlock(u64),
    /// Filesystem blocksize.
    FsBlockSize(u64),
    /// Minimal block size accessible by the filesystem.
    BlockSize(u64),
    /// Endianness of filesystem.
    Endianness(Endianness),
}

pub struct BlockInfo {
    tags: Vec<Tag>,
}

impl BlockInfo {
    pub fn new() -> BlockInfo {
        BlockInfo { tags: Vec::new() }
    }

    pub fn set(&mut self, tag: Tag) {
        self.tags.push(tag);
    }

    pub fn fs_type(&self) -> Option<&BlockType> {
        self.tags.iter().find_map(|t| match t {
            Tag::FsType(t) => Some(t),
            _ => None,
        })
    }

    pub fn sec_type(&self) -> Option<&SecType> {
        self.tags.iter().find_map(|t| match t {
            Tag::SecType(t) => Some(t),
            _ => None,
        })
    }
}

#[derive(Debug)]
pub struct LowProbe<R: Read + Seek> {
    reader: Reader<R>,
    offset: u64,
    size: u64,
    filter: Filter,
}

impl<R: Read + Seek> LowProbe<R> {
    fn new(reader: R, offset: u64, size: u64, filter: Filter) -> LowProbe<R> {
        LowProbe {
            reader: Reader(reader),
            offset,
            size,
            filter,
        }
    }

    fn probe(&self) -> Result<BlockInfo, Error> {
        todo!()
    }
}

bitflags! {
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct Filter: u64 {
        const SKIP_CONT = 1 << 0;
        const SKIP_PT = 1 << 1;
        const SKIP_FS = 1 << 2;
        const SKIP_LUKS1 = 1 << 3;
        const SKIP_LUKS2 = 1 << 4;
        const SKIP_LUKS_OPAL = 1 << 5;
        const SKIP_DOS = 1 << 6;
        const SKIP_GPT = 1 << 7;
        const SKIP_EXFAT = 1 << 8;
        const SKIP_JBD = 1 << 9;
        const SKIP_EXT2 = 1 << 10;
        const SKIP_EXT3 = 1 << 11;
        const SKIP_EXT4 = 1 << 12;
        const SKIP_LINUX_SWAP_V0 = 1 << 13;
        const SKIP_LINUX_SWAP_V1 = 1 << 14;
        const SKIP_SWSUSPEND = 1 << 15;
        const SKIP_NTFS = 1 << 16;
        const SKIP_VFAT = 1 << 17;
        const SKIP_XFS = 1 << 18;
        const SKIP_APFS = 1 << 19;
        const SKIP_SQUASHFS3 = 1 << 20;
        const SKIP_SQUASHFS = 1 << 21;
    }
}
