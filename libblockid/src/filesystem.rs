pub mod apfs;
pub mod exfat;
pub mod ext;
pub mod luks;
pub mod ntfs;
pub mod vfat;
pub mod vxfs;
pub mod xfs;

use bitflags::bitflags;

use crate::{
    error::Error,
    filesystem::{
        apfs::{APFS_MAGICS, APFS_MINSZ, probe_apfs},
        exfat::{EXFAT_MAGICS, EXFAT_MINSZ, probe_exfat},
        ext::{EXT_MAGICS, EXT_MINSZ, probe_ext2, probe_ext3, probe_ext4, probe_jbd},
        luks::{
            LUKS1_MAGICS, LUKS1_MINSZ, LUKS2_MAGICS, LUKS2_MINSZ, LUKSOPAL_MAGICS, probe_luks_opal,
            probe_luks1, probe_luks2,
        },
        ntfs::{NTFS_MAGICS, NTFS_MINSZ, probe_ntfs},
        vfat::{VFAT_MAGICS, VFAT_MINSZ, probe_vfat},
        vxfs::{VXFS_MAGICS, VXFS_MINSZ, probe_vxfs},
        xfs::{XFS_MAGICS, XFS_MINSZ, probe_xfs},
    },
    io::{BlockIo, Reader},
    probe::{Endianness, Id, Magic, ProbeFlags, Usage},
};

/// Order used to detect partition tables in [`probe_block`]
/// 
/// [`probe_block`]: crate::probe::Probe::probe_block
#[rustfmt::skip]
pub const BLOCK_DETECT_ORDER: &[(BlockFilter, BlockType)] = &[
    (BlockFilter::SKIP_APFS, BlockType::Apfs),
    (BlockFilter::SKIP_EXFAT, BlockType::Exfat),
    (BlockFilter::SKIP_JBD, BlockType::Jbd),
    (BlockFilter::SKIP_EXT2, BlockType::Ext2),
    (BlockFilter::SKIP_EXT3, BlockType::Ext3),
    (BlockFilter::SKIP_EXT4, BlockType::Ext4),
    (BlockFilter::SKIP_LUKS1, BlockType::LUKS1),
    (BlockFilter::SKIP_LUKS2, BlockType::LUKS2),
    (BlockFilter::SKIP_LUKS_OPAL, BlockType::LUKSOpal),
    (BlockFilter::SKIP_NTFS, BlockType::Ntfs),
    (BlockFilter::SKIP_VFAT, BlockType::Vfat),
    (BlockFilter::SKIP_VXFS, BlockType::Vxfs),
    (BlockFilter::SKIP_XFS, BlockType::Xfs),
];

/// A generic handler for probing a filesystem type.
#[derive(Debug, Copy, Clone, Hash)]
pub struct BlockHandler<IO: BlockIo> {
    /// Minimum disk size in bytes required for filesystem, if any.
    pub minsz: Option<u64>,
    /// Minimum disk size in bytes required for this filesystem, if any.
    pub magics: Option<&'static [Magic]>,
    /// Probes the filesystem, returning its info on success.
    #[allow(clippy::type_complexity)]
    pub probe: fn(&mut Reader<IO>, ProbeFlags, u64, Magic) -> Result<BlockInfo, Error<IO::Error>>,
}

/// The type of filesystem supported.
#[non_exhaustive]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BlockType {
    Apfs,
    Exfat,
    Jbd,
    Ext2,
    Ext3,
    Ext4,
    LUKS1,
    LUKS2,
    LUKSOpal,
    Ntfs,
    Vfat,
    Vxfs,
    Xfs,
}

impl BlockType {
    pub(crate) fn block_handler<IO: BlockIo>(&self) -> BlockHandler<IO> {
        match self {
            BlockType::LUKS1 => BlockHandler {
                minsz: LUKS1_MINSZ,
                magics: LUKS1_MAGICS,
                probe: probe_luks1,
            },

            BlockType::LUKS2 => BlockHandler {
                minsz: LUKS2_MINSZ,
                magics: LUKS2_MAGICS,
                probe: probe_luks2,
            },

            BlockType::LUKSOpal => BlockHandler {
                minsz: LUKS2_MINSZ,
                magics: LUKSOPAL_MAGICS,
                probe: probe_luks_opal,
            },
            BlockType::Exfat => BlockHandler {
                minsz: EXFAT_MINSZ,
                magics: EXFAT_MAGICS,
                probe: probe_exfat,
            },
            BlockType::Jbd => BlockHandler {
                minsz: EXT_MINSZ,
                magics: EXT_MAGICS,
                probe: probe_jbd,
            },
            BlockType::Apfs => BlockHandler {
                minsz: APFS_MINSZ,
                magics: APFS_MAGICS,
                probe: probe_apfs,
            },
            BlockType::Ext2 => BlockHandler {
                minsz: EXT_MINSZ,
                magics: EXT_MAGICS,
                probe: probe_ext2,
            },
            BlockType::Ext3 => BlockHandler {
                minsz: EXT_MINSZ,
                magics: EXT_MAGICS,
                probe: probe_ext3,
            },
            BlockType::Ext4 => BlockHandler {
                minsz: EXT_MINSZ,
                magics: EXT_MAGICS,
                probe: probe_ext4,
            },
            BlockType::Ntfs => BlockHandler {
                minsz: NTFS_MINSZ,
                magics: NTFS_MAGICS,
                probe: probe_ntfs,
            },
            BlockType::Vfat => BlockHandler {
                minsz: VFAT_MINSZ,
                magics: VFAT_MAGICS,
                probe: probe_vfat,
            },
            BlockType::Vxfs => BlockHandler {
                minsz: VXFS_MINSZ,
                magics: VXFS_MAGICS,
                probe: probe_vxfs,
            },
            BlockType::Xfs => BlockHandler {
                minsz: XFS_MINSZ,
                magics: XFS_MAGICS,
                probe: probe_xfs,
            },
            _ => todo!(),
        }
    }
}

/// The subtype of filesystems.
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

    pub(crate) fn set(&mut self, tag: BlockTag) {
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
        const SKIP_APFS = 1 << 0;
        const SKIP_EXFAT = 1 << 1;
        const SKIP_JBD = 1 << 2;
        const SKIP_EXT2 = 1 << 3;
        const SKIP_EXT3 = 1 << 4;
        const SKIP_EXT4 = 1 << 5;
        const SKIP_LUKS1 = 1 << 6;
        const SKIP_LUKS2 = 1 << 7;
        const SKIP_LUKS_OPAL = 1 << 8;
        const SKIP_NTFS = 1 << 9;
        const SKIP_VFAT = 1 << 10;
        const SKIP_VXFS = 1 << 11;
        const SKIP_XFS = 1 << 12;
    }
}
