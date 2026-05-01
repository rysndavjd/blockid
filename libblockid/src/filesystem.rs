pub mod exfat;
pub mod ext;
pub mod luks;
pub mod vfat;

use bitflags::bitflags;

use crate::{
    error::Error,
    filesystem::{
        exfat::{EXFAT_MAGICS, probe_exfat},
        ext::{EXT_MAGICS, probe_ext2, probe_ext3, probe_ext4, probe_jbd},
        luks::{
            LUKS1_MAGICS, LUKS2_MAGICS, LUKSOPAL_MAGICS, probe_luks_opal, probe_luks1, probe_luks2,
        },
        vfat::{VFAT_MAGICS, probe_vfat},
    },
    io::{BlockIo, Reader},
    probe::{Endianness, Id, Magic, Usage},
};

#[rustfmt::skip]
pub const BLOCK_DETECT_ORDER: &[(BlockFilter, BlockType)] = &[
    (BlockFilter::SKIP_LUKS1, BlockType::LUKS1),
    (BlockFilter::SKIP_LUKS2, BlockType::LUKS2),
    (BlockFilter::SKIP_LUKS_OPAL, BlockType::LUKSOpal),
    (BlockFilter::SKIP_EXFAT, BlockType::Exfat),
    (BlockFilter::SKIP_JBD, BlockType::Jbd),
    (BlockFilter::SKIP_EXT2, BlockType::Ext2),
    (BlockFilter::SKIP_EXT3, BlockType::Ext3),
    (BlockFilter::SKIP_EXT4, BlockType::Ext4),
    (BlockFilter::SKIP_VFAT, BlockType::Vfat),
];

#[derive(Debug, Copy, Clone, Hash)]
pub struct BlockHandler<IO: BlockIo> {
    pub minsz: Option<u64>,
    pub magics: Option<&'static [Magic]>,
    #[allow(clippy::type_complexity)]
    pub probe: fn(&mut Reader<IO>, u64, Magic) -> Result<BlockInfo, Error<IO::Error>>,
}

#[non_exhaustive]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BlockType {
    LUKS1,
    LUKS2,
    LUKSOpal,
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
    pub fn block_handler<IO: BlockIo>(&self) -> BlockHandler<IO> {
        match self {
            BlockType::LUKS1 => BlockHandler {
                minsz: None,
                magics: LUKS1_MAGICS,
                probe: probe_luks1,
            },

            BlockType::LUKS2 => BlockHandler {
                minsz: None,
                magics: LUKS2_MAGICS,
                probe: probe_luks2,
            },

            BlockType::LUKSOpal => BlockHandler {
                minsz: None,
                magics: LUKSOPAL_MAGICS,
                probe: probe_luks_opal,
            },
            BlockType::Exfat => BlockHandler {
                minsz: None,
                magics: EXFAT_MAGICS,
                probe: probe_exfat,
            },
            BlockType::Jbd => BlockHandler {
                minsz: None,
                magics: EXT_MAGICS,
                probe: probe_jbd,
            },
            BlockType::Ext2 => BlockHandler {
                minsz: None,
                magics: EXT_MAGICS,
                probe: probe_ext2,
            },
            BlockType::Ext3 => BlockHandler {
                minsz: None,
                magics: EXT_MAGICS,
                probe: probe_ext3,
            },
            BlockType::Ext4 => BlockHandler {
                minsz: None,
                magics: EXT_MAGICS,
                probe: probe_ext4,
            },
            BlockType::Vfat => BlockHandler {
                minsz: None,
                magics: VFAT_MAGICS,
                probe: probe_vfat,
            },
            _ => todo!(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SubType {
    Fat12,
    Fat16,
    Fat32,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockTag {
    /// Block type, Eg: EXT4.
    BlockType(BlockType),
    /// Sub block type, Eg: Filsystem is VFAT but subtype is FAT16.
    SubType(SubType),
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
    /// OS used to create filesystem.
    Creator(String),
}

#[derive(Debug)]
pub struct BlockInfo {
    tags: Vec<BlockTag>,
}

impl BlockInfo {
    pub(crate) fn new() -> BlockInfo {
        BlockInfo { tags: Vec::new() }
    }

    pub fn inner(&self) -> &Vec<BlockTag> {
        &self.tags
    }

    pub fn into_inner(self) -> Vec<BlockTag> {
        self.tags
    }

    pub fn set(&mut self, tag: BlockTag) {
        self.tags.push(tag);
    }

    pub fn block_type(&self) -> Option<BlockType> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::BlockType(t) => Some(*t),
            _ => None,
        })
    }

    pub fn sub_type(&self) -> Option<SubType> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::SubType(t) => Some(*t),
            _ => None,
        })
    }

    pub fn label(&self) -> Option<&String> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::Label(t) => Some(t),
            _ => None,
        })
    }

    pub fn id(&self) -> Option<Id> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::Id(t) => Some(*t),
            _ => None,
        })
    }

    pub fn sub_member_id(&self) -> Option<Id> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::SubMemberId(t) => Some(*t),
            _ => None,
        })
    }

    pub fn ext_log_id(&self) -> Option<Id> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::ExtLogId(t) => Some(*t),
            _ => None,
        })
    }

    pub fn ext_journal_id(&self) -> Option<Id> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::ExtJournalId(t) => Some(*t),
            _ => None,
        })
    }

    pub fn usage(&self) -> Option<Usage> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::Usage(t) => Some(*t),
            _ => None,
        })
    }

    pub fn version(&self) -> Option<&String> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::Version(t) => Some(t),
            _ => None,
        })
    }

    pub fn magic(&self) -> Option<&Vec<u8>> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::Magic(t) => Some(t),
            _ => None,
        })
    }

    pub fn magic_offset(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::MagicOffset(t) => Some(*t),
            _ => None,
        })
    }

    pub fn fs_size(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::FsSize(t) => Some(*t),
            _ => None,
        })
    }

    pub fn fs_last_block(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::FsLastBlock(t) => Some(*t),
            _ => None,
        })
    }

    pub fn fs_block_size(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::FsBlockSize(t) => Some(*t),
            _ => None,
        })
    }

    pub fn block_size(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::BlockSize(t) => Some(*t),
            _ => None,
        })
    }

    pub fn endianness(&self) -> Option<Endianness> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::Endianness(t) => Some(*t),
            _ => None,
        })
    }

    pub fn creator(&self) -> Option<&String> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::Creator(t) => Some(t),
            _ => None,
        })
    }
}

bitflags! {
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct BlockFilter: u64 {
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
