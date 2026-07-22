pub(crate) mod apfs;
pub(crate) mod cramfs;
pub(crate) mod exfat;
pub(crate) mod ext;
pub(crate) mod luks;
pub(crate) mod ntfs;
pub(crate) mod vfat;
pub(crate) mod vxfs;
pub(crate) mod xfs;

use bitflags::bitflags;
use fat_volume_id::{id32::VolumeId32, id64::VolumeId64};
use uuid::Uuid;

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
    probe::{Endianness, Magic, ProbeFlags, Usage},
    std::fmt,
};

/// Order used to detect partition tables
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
#[derive(Debug, Copy, Clone)]
pub(crate) struct BlockHandler<IO: BlockIo> {
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
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
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

impl fmt::Display for BlockType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockType::Apfs => write!(f, "apfs"),
            BlockType::Exfat => write!(f, "exfat"),
            BlockType::Jbd => write!(f, "jbd"),
            BlockType::Ext2 => write!(f, "ext2"),
            BlockType::Ext3 => write!(f, "ext3"),
            BlockType::Ext4 => write!(f, "ext4"),
            BlockType::LUKS1 => write!(f, "luks1"),
            BlockType::LUKS2 => write!(f, "luks2"),
            BlockType::LUKSOpal => write!(f, "luks_opal"),
            BlockType::Ntfs => write!(f, "ntfs"),
            BlockType::Vfat => write!(f, "vfat"),
            BlockType::Vxfs => write!(f, "vxfs"),
            BlockType::Xfs => write!(f, "xfs"),
        }
    }
}

impl BlockType {
    pub(crate) const fn block_handler<IO: BlockIo>(&self) -> BlockHandler<IO> {
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
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum FilesystemId {
    /// A 128-bit universally unique identifier.
    Uuid(Uuid),
    /// A 32-bit volume serial number.
    VolumeId32(VolumeId32),
    /// A 64-bit volume serial number.
    VolumeId64(VolumeId64),
}

impl FilesystemId {
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            FilesystemId::Uuid(t) => Some(*t),
            _ => None,
        }
    }

    pub fn as_volumeid32(&self) -> Option<VolumeId32> {
        match self {
            FilesystemId::VolumeId32(t) => Some(*t),
            _ => None,
        }
    }

    pub fn as_volumeid64(&self) -> Option<VolumeId64> {
        match self {
            FilesystemId::VolumeId64(t) => Some(*t),
            _ => None,
        }
    }
}

impl From<Uuid> for FilesystemId {
    fn from(value: Uuid) -> Self {
        FilesystemId::Uuid(value)
    }
}

impl From<VolumeId32> for FilesystemId {
    fn from(value: VolumeId32) -> Self {
        FilesystemId::VolumeId32(value)
    }
}

impl From<VolumeId64> for FilesystemId {
    fn from(value: VolumeId64) -> Self {
        FilesystemId::VolumeId64(value)
    }
}

/// The subtype of filesystems.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SubType {
    Fat12,
    Fat16,
    Fat32,
}

#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockTag {
    /// Block type, Eg: EXT4.
    BlockType(BlockType),
    /// Sub block type, Eg: Filsystem is VFAT but subtype is FAT16.
    SubType(SubType),
    /// Filesystem label, Eg: `LABEL`.
    Label(String),
    /// Filesystem identifier.
    /// Eg:
    ///     UUID: `67e55044-10b1-426f-9247-bb680e5fe0c8`
    ///     VolumeId32: `2a9d-b913`
    ///     VolumeId64: `17acf19235bcde78`
    FilesystemId(FilesystemId),
    /// Sub member identifier.
    SubMemberId(Uuid),
    /// External log identifier.
    ExtLogId(Uuid),
    /// External journal identifier.
    ExtJournalId(Uuid),
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

    pub fn inner(&self) -> &[BlockTag] {
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

    pub fn filesystem_id(&self) -> Option<FilesystemId> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::FilesystemId(t) => Some(*t),
            _ => None,
        })
    }

    pub fn sub_member_id(&self) -> Option<Uuid> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::SubMemberId(t) => Some(*t),
            _ => None,
        })
    }

    pub fn ext_log_id(&self) -> Option<Uuid> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::ExtLogId(t) => Some(*t),
            _ => None,
        })
    }

    pub fn ext_journal_id(&self) -> Option<Uuid> {
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

    pub fn magic(&self) -> Option<&[u8]> {
        self.tags.iter().find_map(|t| match t {
            BlockTag::Magic(t) => Some(t.as_slice()),
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

#[cfg(feature = "serde")]
impl serde::Serialize for BlockInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(self.tags.len()))?;

        for tag in &self.tags {
            match tag {
                BlockTag::BlockType(fs) => {
                    map.serialize_entry("FS_TYPE", fs)?;
                }
                BlockTag::SubType(sub) => {
                    map.serialize_entry("FS_SUB_TYPE", sub)?;
                }
                BlockTag::Label(label) => {
                    map.serialize_entry("FS_LABEL", label)?;
                }
                BlockTag::FilesystemId(id) => match id {
                    FilesystemId::Uuid(uuid) => {
                        map.serialize_entry("FS_ID", uuid)?;
                    }
                    FilesystemId::VolumeId32(id32) => {
                        map.serialize_entry("FS_ID", id32)?;
                    }
                    FilesystemId::VolumeId64(id64) => {
                        map.serialize_entry("FS_ID", id64)?;
                    }
                },
                BlockTag::SubMemberId(id) => {
                    map.serialize_entry("FS_SUB_MEMBER_ID", id)?;
                }
                BlockTag::ExtLogId(id) => {
                    map.serialize_entry("FS_EXT_LOG_ID", id)?;
                }
                BlockTag::ExtJournalId(id) => {
                    map.serialize_entry("FS_JOURNAL_ID", id)?;
                }
                BlockTag::Usage(usage) => {
                    map.serialize_entry("FS_USAGE", usage)?;
                }
                BlockTag::Version(ver) => {
                    map.serialize_entry("FS_VERSION", ver)?;
                }
                BlockTag::Magic(mag) => {
                    map.serialize_entry("FS_MAGIC", mag)?;
                }
                BlockTag::MagicOffset(off) => {
                    map.serialize_entry("FS_MAGIC_OFFSET", off)?;
                }
                BlockTag::FsSize(sz) => {
                    map.serialize_entry("FS_SIZE", sz)?;
                }
                BlockTag::FsLastBlock(last_block) => {
                    map.serialize_entry("FS_LAST_BLOCK", last_block)?;
                }
                BlockTag::FsBlockSize(blk_sz) => {
                    map.serialize_entry("FS_BLOCK_SIZE", blk_sz)?;
                }
                BlockTag::BlockSize(blk_sz) => {
                    map.serialize_entry("BLOCK_SIZE", blk_sz)?;
                }
                BlockTag::Endianness(endian) => {
                    map.serialize_entry("FS_ENDIANNESS", endian)?;
                }
                BlockTag::Creator(creator) => {
                    map.serialize_entry("FS_CREATOR", creator)?;
                }
            }
        }

        map.end()
    }
}

bitflags! {
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
