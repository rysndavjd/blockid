use bitflags::bitflags;
use fat_volume_id::{VolumeId32, VolumeId64};
use uuid::Uuid;

use crate::{
    error::{Error, ErrorKind},
    // filesystem::vfat::{VFAT_MAGICS, probe_vfat},
    filesystem::ext::{EXT_MAGICS, probe_ext2, probe_ext3, probe_ext4, probe_jbd},
    io::{BlockIo, Reader, SeekFrom},
};

const BLOCK_DETECT_ORDER: &[BlockType] = &[
    BlockType::Jbd,
    BlockType::Ext2,
    BlockType::Ext3,
    BlockType::Ext4,
];

#[derive(Debug, Copy, Clone, Hash)]
pub struct SuperblockInfo<IO: BlockIo> {
    pub minsz: Option<u64>,
    pub magics: Option<&'static [Magic]>,
    #[allow(clippy::type_complexity)]
    pub probe: fn(&mut Reader<IO>, u64, Magic) -> Result<BlockInfo, Error<IO>>,
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
    fn block_info<IO: BlockIo>(&self) -> SuperblockInfo<IO> {
        match self {
            // BlockType::Vfat => SuperblockInfo {
            //     minsz: None,
            //     magics: VFAT_MAGICS,
            //     probe: probe_vfat,
            // },
            BlockType::Ext2 => SuperblockInfo {
                minsz: None,
                magics: EXT_MAGICS,
                probe: probe_ext2,
            },
            BlockType::Ext3 => SuperblockInfo {
                minsz: None,
                magics: EXT_MAGICS,
                probe: probe_ext3,
            },
            BlockType::Ext4 => SuperblockInfo {
                minsz: None,
                magics: EXT_MAGICS,
                probe: probe_ext4,
            },
            BlockType::Jbd => SuperblockInfo {
                minsz: None,
                magics: EXT_MAGICS,
                probe: probe_jbd,
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

#[derive(Debug)]
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
pub struct LowProbe<IO: BlockIo> {
    reader: Reader<IO>,
    offset: u64,
    filter: Filter,
}

impl<IO: BlockIo> LowProbe<IO> {
    pub fn new(reader: IO, offset: u64, filter: Filter) -> LowProbe<IO> {
        LowProbe {
            reader: Reader::new(reader),
            offset,
            filter,
        }
    }

    fn get_magic(&mut self, magics: Option<&'static [Magic]>) -> Result<Magic, Error<IO>> {
        if let Some(magics) = magics {
            let mut buf = [0u8; 16];

            for magic in magics {
                self.reader
                    .seek(SeekFrom::Start(magic.b_offset))?;

                self.reader
                    .read_exact(&mut buf[..magic.len])
                    .map_err(|e| ErrorKind::IoError::<IO>(e))?;

                if &buf[..magic.len] == magic.magic {
                    return Ok(*magic);
                }
            }
        }
        Ok(Magic::EMPTY_MAGIC)
    }

    pub fn probe(&mut self) -> Result<BlockInfo, Error<IO>> {
        for block in BLOCK_DETECT_ORDER {
            let info = block.block_info::<IO>();
            let magic = match self.get_magic(info.magics) {
                Ok(t) => t,
                Err(_) => continue,
            };

            match (info.probe)(&mut self.reader, self.offset, magic) {
                Ok(t) => return Ok(t),
                Err(_) => continue,
            };
        }
        return Err(ErrorKind::ProbesExhausted.into());
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
