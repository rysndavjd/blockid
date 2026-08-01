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

/// Order used to detect filesystems
#[rustfmt::skip]
pub const FS_DETECT_ORDER: &[(FsFilter, FsType)] = &[
    (FsFilter::SKIP_APFS, FsType::Apfs),
    (FsFilter::SKIP_EXFAT, FsType::Exfat),
    (FsFilter::SKIP_JBD, FsType::Jbd),
    (FsFilter::SKIP_EXT2, FsType::Ext2),
    (FsFilter::SKIP_EXT3, FsType::Ext3),
    (FsFilter::SKIP_EXT4, FsType::Ext4),
    (FsFilter::SKIP_LUKS1, FsType::LUKS1),
    (FsFilter::SKIP_LUKS2, FsType::LUKS2),
    (FsFilter::SKIP_LUKS_OPAL, FsType::LUKSOpal),
    (FsFilter::SKIP_NTFS, FsType::Ntfs),
    (FsFilter::SKIP_VFAT, FsType::Vfat),
    (FsFilter::SKIP_VXFS, FsType::Vxfs),
    (FsFilter::SKIP_XFS, FsType::Xfs),
];

/// A generic handler for probing a filesystem type.
#[derive(Debug, Copy, Clone)]
pub(crate) struct FsHandler<IO: BlockIo> {
    /// Minimum disk size in bytes required for filesystem, if any.
    pub minsz: Option<u64>,
    /// Minimum disk size in bytes required for this filesystem, if any.
    pub magics: Option<&'static [Magic]>,
    /// Probes the filesystem, returning its info on success.
    #[allow(clippy::type_complexity)]
    pub probe: fn(&mut Reader<IO>, ProbeFlags, u64, Magic) -> Result<FsInfo, Error<IO::Error>>,
}

/// The type of filesystem supported.
#[non_exhaustive]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum FsType {
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

impl fmt::Display for FsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FsType::Apfs => write!(f, "apfs"),
            FsType::Exfat => write!(f, "exfat"),
            FsType::Jbd => write!(f, "jbd"),
            FsType::Ext2 => write!(f, "ext2"),
            FsType::Ext3 => write!(f, "ext3"),
            FsType::Ext4 => write!(f, "ext4"),
            FsType::LUKS1 => write!(f, "luks1"),
            FsType::LUKS2 => write!(f, "luks2"),
            FsType::LUKSOpal => write!(f, "luks_opal"),
            FsType::Ntfs => write!(f, "ntfs"),
            FsType::Vfat => write!(f, "vfat"),
            FsType::Vxfs => write!(f, "vxfs"),
            FsType::Xfs => write!(f, "xfs"),
        }
    }
}

impl FsType {
    pub(crate) const fn fs_handler<IO: BlockIo>(&self) -> FsHandler<IO> {
        match self {
            FsType::LUKS1 => FsHandler {
                minsz: LUKS1_MINSZ,
                magics: LUKS1_MAGICS,
                probe: probe_luks1,
            },

            FsType::LUKS2 => FsHandler {
                minsz: LUKS2_MINSZ,
                magics: LUKS2_MAGICS,
                probe: probe_luks2,
            },

            FsType::LUKSOpal => FsHandler {
                minsz: LUKS2_MINSZ,
                magics: LUKSOPAL_MAGICS,
                probe: probe_luks_opal,
            },
            FsType::Exfat => FsHandler {
                minsz: EXFAT_MINSZ,
                magics: EXFAT_MAGICS,
                probe: probe_exfat,
            },
            FsType::Jbd => FsHandler {
                minsz: EXT_MINSZ,
                magics: EXT_MAGICS,
                probe: probe_jbd,
            },
            FsType::Apfs => FsHandler {
                minsz: APFS_MINSZ,
                magics: APFS_MAGICS,
                probe: probe_apfs,
            },
            FsType::Ext2 => FsHandler {
                minsz: EXT_MINSZ,
                magics: EXT_MAGICS,
                probe: probe_ext2,
            },
            FsType::Ext3 => FsHandler {
                minsz: EXT_MINSZ,
                magics: EXT_MAGICS,
                probe: probe_ext3,
            },
            FsType::Ext4 => FsHandler {
                minsz: EXT_MINSZ,
                magics: EXT_MAGICS,
                probe: probe_ext4,
            },
            FsType::Ntfs => FsHandler {
                minsz: NTFS_MINSZ,
                magics: NTFS_MAGICS,
                probe: probe_ntfs,
            },
            FsType::Vfat => FsHandler {
                minsz: VFAT_MINSZ,
                magics: VFAT_MAGICS,
                probe: probe_vfat,
            },
            FsType::Vxfs => FsHandler {
                minsz: VXFS_MINSZ,
                magics: VXFS_MAGICS,
                probe: probe_vxfs,
            },
            FsType::Xfs => FsHandler {
                minsz: XFS_MINSZ,
                magics: XFS_MAGICS,
                probe: probe_xfs,
            },
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum FsId {
    /// A 128-bit universally unique identifier.
    Uuid(Uuid),
    /// A 32-bit volume serial number.
    VolumeId32(VolumeId32),
    /// A 64-bit volume serial number.
    VolumeId64(VolumeId64),
}

impl FsId {
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            FsId::Uuid(t) => Some(*t),
            _ => None,
        }
    }

    pub fn as_volumeid32(&self) -> Option<VolumeId32> {
        match self {
            FsId::VolumeId32(t) => Some(*t),
            _ => None,
        }
    }

    pub fn as_volumeid64(&self) -> Option<VolumeId64> {
        match self {
            FsId::VolumeId64(t) => Some(*t),
            _ => None,
        }
    }
}

impl From<Uuid> for FsId {
    fn from(value: Uuid) -> Self {
        FsId::Uuid(value)
    }
}

impl From<VolumeId32> for FsId {
    fn from(value: VolumeId32) -> Self {
        FsId::VolumeId32(value)
    }
}

impl From<VolumeId64> for FsId {
    fn from(value: VolumeId64) -> Self {
        FsId::VolumeId64(value)
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
pub enum FsTag {
    /// Block type, Eg: EXT4.
    FsType(FsType),
    /// Sub block type, Eg: Filsystem is VFAT but subtype is FAT16.
    SubType(SubType),
    /// Filesystem label, Eg: `LABEL`.
    Label(String),
    /// Filesystem identifier.
    /// Eg:
    ///     UUID: `67e55044-10b1-426f-9247-bb680e5fe0c8`
    ///     VolumeId32: `2a9d-b913`
    ///     VolumeId64: `17acf19235bcde78`
    FsId(FsId),
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
pub struct FsInfo {
    tags: Vec<FsTag>,
}

impl FsInfo {
    pub(crate) fn new() -> FsInfo {
        FsInfo { tags: Vec::new() }
    }

    pub fn inner(&self) -> &[FsTag] {
        &self.tags
    }

    pub fn into_inner(self) -> Vec<FsTag> {
        self.tags
    }

    pub(crate) fn set(&mut self, tag: FsTag) {
        self.tags.push(tag);
    }

    pub fn fs_type(&self) -> Option<FsType> {
        self.tags.iter().find_map(|t| match t {
            FsTag::FsType(t) => Some(*t),
            _ => None,
        })
    }

    pub fn sub_type(&self) -> Option<SubType> {
        self.tags.iter().find_map(|t| match t {
            FsTag::SubType(t) => Some(*t),
            _ => None,
        })
    }

    pub fn label(&self) -> Option<&String> {
        self.tags.iter().find_map(|t| match t {
            FsTag::Label(t) => Some(t),
            _ => None,
        })
    }

    pub fn fs_id(&self) -> Option<FsId> {
        self.tags.iter().find_map(|t| match t {
            FsTag::FsId(t) => Some(*t),
            _ => None,
        })
    }

    pub fn sub_member_id(&self) -> Option<Uuid> {
        self.tags.iter().find_map(|t| match t {
            FsTag::SubMemberId(t) => Some(*t),
            _ => None,
        })
    }

    pub fn ext_log_id(&self) -> Option<Uuid> {
        self.tags.iter().find_map(|t| match t {
            FsTag::ExtLogId(t) => Some(*t),
            _ => None,
        })
    }

    pub fn ext_journal_id(&self) -> Option<Uuid> {
        self.tags.iter().find_map(|t| match t {
            FsTag::ExtJournalId(t) => Some(*t),
            _ => None,
        })
    }

    pub fn usage(&self) -> Option<Usage> {
        self.tags.iter().find_map(|t| match t {
            FsTag::Usage(t) => Some(*t),
            _ => None,
        })
    }

    pub fn version(&self) -> Option<&String> {
        self.tags.iter().find_map(|t| match t {
            FsTag::Version(t) => Some(t),
            _ => None,
        })
    }

    pub fn magic(&self) -> Option<&[u8]> {
        self.tags.iter().find_map(|t| match t {
            FsTag::Magic(t) => Some(t.as_slice()),
            _ => None,
        })
    }

    pub fn magic_offset(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            FsTag::MagicOffset(t) => Some(*t),
            _ => None,
        })
    }

    pub fn fs_size(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            FsTag::FsSize(t) => Some(*t),
            _ => None,
        })
    }

    pub fn fs_last_block(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            FsTag::FsLastBlock(t) => Some(*t),
            _ => None,
        })
    }

    pub fn fs_block_size(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            FsTag::FsBlockSize(t) => Some(*t),
            _ => None,
        })
    }

    pub fn block_size(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            FsTag::BlockSize(t) => Some(*t),
            _ => None,
        })
    }

    pub fn endianness(&self) -> Option<Endianness> {
        self.tags.iter().find_map(|t| match t {
            FsTag::Endianness(t) => Some(*t),
            _ => None,
        })
    }

    pub fn creator(&self) -> Option<&String> {
        self.tags.iter().find_map(|t| match t {
            FsTag::Creator(t) => Some(t),
            _ => None,
        })
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for FsInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(self.tags.len()))?;

        for tag in &self.tags {
            match tag {
                FsTag::FsType(fs) => {
                    map.serialize_entry("FS_TYPE", fs)?;
                }
                FsTag::SubType(sub) => {
                    map.serialize_entry("SUB_TYPE", sub)?;
                }
                FsTag::Label(label) => {
                    map.serialize_entry("LABEL", label)?;
                }
                FsTag::FsId(id) => match id {
                    FsId::Uuid(uuid) => {
                        map.serialize_entry("FS_ID", uuid)?;
                    }
                    FsId::VolumeId32(id32) => {
                        map.serialize_entry("FS_ID", id32)?;
                    }
                    FsId::VolumeId64(id64) => {
                        map.serialize_entry("FS_ID", id64)?;
                    }
                },
                FsTag::SubMemberId(id) => {
                    map.serialize_entry("SUB_MEMBER_ID", id)?;
                }
                FsTag::ExtLogId(id) => {
                    map.serialize_entry("EXT_LOG_ID", id)?;
                }
                FsTag::ExtJournalId(id) => {
                    map.serialize_entry("EXT_JOURNAL_ID", id)?;
                }
                FsTag::Usage(usage) => {
                    map.serialize_entry("USAGE", usage)?;
                }
                FsTag::Version(ver) => {
                    map.serialize_entry("VERSION", ver)?;
                }
                FsTag::Magic(mag) => {
                    map.serialize_entry("MAGIC", mag)?;
                }
                FsTag::MagicOffset(off) => {
                    map.serialize_entry("MAGIC_OFFSET", off)?;
                }
                FsTag::FsSize(sz) => {
                    map.serialize_entry("FS_SIZE", sz)?;
                }
                FsTag::FsLastBlock(last_block) => {
                    map.serialize_entry("FS_LAST_BLOCK", last_block)?;
                }
                FsTag::FsBlockSize(blk_sz) => {
                    map.serialize_entry("FS_BLOCK_SIZE", blk_sz)?;
                }
                FsTag::BlockSize(blk_sz) => {
                    map.serialize_entry("BLOCK_SIZE", blk_sz)?;
                }
                FsTag::Endianness(endian) => {
                    map.serialize_entry("ENDIANNESS", endian)?;
                }
                FsTag::Creator(creator) => {
                    map.serialize_entry("CREATOR", creator)?;
                }
            }
        }

        map.end()
    }
}

bitflags! {
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct FsFilter: u64 {
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
