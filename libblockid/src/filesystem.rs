pub(crate) mod apfs;
pub(crate) mod cramfs;
pub(crate) mod exfat;
pub(crate) mod ext;
pub(crate) mod luks;
pub(crate) mod ntfs;
pub(crate) mod squashfs;
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
        cramfs::{CRAMFS_MAGICS, CRAMFS_MINSZ, probe_cramfs},
        exfat::{EXFAT_MAGICS, EXFAT_MINSZ, probe_exfat},
        ext::{EXT_MAGICS, EXT_MINSZ, probe_ext2, probe_ext3, probe_ext4, probe_jbd},
        luks::{LUKS_MAGICS, LUKS1_MINSZ, LUKS2_MINSZ, probe_luks},
        ntfs::{NTFS_MAGICS, NTFS_MINSZ, probe_ntfs},
        squashfs::{
            SQUASHFS_MAGICS, SQUASHFS_MINSZ, SQUASHFS3_MAGICS, probe_squashfs, probe_squashfs3,
        },
        vfat::{VFAT_MAGICS, VFAT_MINSZ, probe_vfat},
        vxfs::{VXFS_MAGICS, VXFS_MINSZ, probe_vxfs},
        xfs::{XFS_MAGICS, XFS_MINSZ, probe_xfs},
    },
    io::{BlockIo, Reader},
    probe::{Endianness, Label, Magic},
    std::fmt,
};

/// Order used to detect filesystems
#[rustfmt::skip]
pub const FS_DETECT_ORDER: &[(FsFilter, FsType)] = &[
    (FsFilter::SKIP_APFS, FsType::Apfs),
    (FsFilter::SKIP_CRAMFS, FsType::Cramfs),
    (FsFilter::SKIP_EXFAT, FsType::Exfat),
    (FsFilter::SKIP_JBD, FsType::Jbd),
    (FsFilter::SKIP_EXT2, FsType::Ext2),
    (FsFilter::SKIP_EXT3, FsType::Ext3),
    (FsFilter::SKIP_EXT4, FsType::Ext4),
    (FsFilter::SKIP_LUKS1, FsType::LUKS1),
    (FsFilter::SKIP_LUKS2, FsType::LUKS2),
    (FsFilter::SKIP_NTFS, FsType::Ntfs),
    (FsFilter::SKIP_SQUASHFS, FsType::Squashfs),
    (FsFilter::SKIP_SQUASHFS3, FsType::Squashfs3),
    (FsFilter::SKIP_VFAT, FsType::Vfat),
    (FsFilter::SKIP_VXFS, FsType::Vxfs),
    (FsFilter::SKIP_XFS, FsType::Xfs),
];

/// A generic handler for probing a filesystem type.
#[derive(Debug, Copy, Clone)]
pub(crate) struct FsHandler<IO: BlockIo> {
    /// Minimum disk size in bytes required for filesystem, if any.
    pub minsz: Option<u64>,
    /// Magic signatures used to identify filesystem, if any.
    pub magics: Option<&'static [Magic]>,
    /// Probes the filesystem, returning its info on success.
    #[allow(clippy::type_complexity)]
    pub probe: fn(&mut Reader<IO>, u64, Magic) -> Result<FsInfo, Error<IO::Error>>,
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
    Cramfs,
    Exfat,
    Jbd,
    Ext2,
    Ext3,
    Ext4,
    LUKS1,
    LUKS2,
    Ntfs,
    Squashfs,
    Squashfs3,
    Vfat,
    Vxfs,
    Xfs,
}

impl fmt::Display for FsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FsType::Apfs => write!(f, "apfs"),
            FsType::Cramfs => write!(f, "cramfs"),
            FsType::Exfat => write!(f, "exfat"),
            FsType::Jbd => write!(f, "jbd"),
            FsType::Ext2 => write!(f, "ext2"),
            FsType::Ext3 => write!(f, "ext3"),
            FsType::Ext4 => write!(f, "ext4"),
            FsType::LUKS1 => write!(f, "luks1"),
            FsType::LUKS2 => write!(f, "luks2"),
            FsType::Ntfs => write!(f, "ntfs"),
            FsType::Squashfs => write!(f, "squashfs"),
            FsType::Squashfs3 => write!(f, "squashfs3"),
            FsType::Vfat => write!(f, "vfat"),
            FsType::Vxfs => write!(f, "vxfs"),
            FsType::Xfs => write!(f, "xfs"),
        }
    }
}

impl FsType {
    pub(crate) const fn fs_handler<IO: BlockIo>(&self) -> FsHandler<IO> {
        match self {
            FsType::Apfs => FsHandler {
                minsz: APFS_MINSZ,
                magics: APFS_MAGICS,
                probe: probe_apfs,
            },
            FsType::Cramfs => FsHandler {
                minsz: CRAMFS_MINSZ,
                magics: CRAMFS_MAGICS,
                probe: probe_cramfs,
            },
            FsType::Exfat => FsHandler {
                minsz: EXFAT_MINSZ,
                magics: EXFAT_MAGICS,
                probe: probe_exfat,
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
            FsType::Jbd => FsHandler {
                minsz: EXT_MINSZ,
                magics: EXT_MAGICS,
                probe: probe_jbd,
            },
            FsType::LUKS1 => FsHandler {
                minsz: LUKS1_MINSZ,
                magics: LUKS_MAGICS,
                probe: probe_luks,
            },
            FsType::LUKS2 => FsHandler {
                minsz: LUKS2_MINSZ,
                magics: LUKS_MAGICS,
                probe: probe_luks,
            },
            FsType::Ntfs => FsHandler {
                minsz: NTFS_MINSZ,
                magics: NTFS_MAGICS,
                probe: probe_ntfs,
            },
            FsType::Squashfs => FsHandler {
                minsz: SQUASHFS_MINSZ,
                magics: SQUASHFS_MAGICS,
                probe: probe_squashfs,
            },
            FsType::Squashfs3 => FsHandler {
                minsz: SQUASHFS_MINSZ,
                magics: SQUASHFS3_MAGICS,
                probe: probe_squashfs3,
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

/// Identifier used by a filesystem to uniquely identify them.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum FsId {
    /// A 128-bit universally unique identifier or [UUID](https://en.wikipedia.org/wiki/Universally_unique_identifier).
    Uuid(Uuid),
    /// A 32-bit volume serial number.
    VolumeId32(VolumeId32),
    /// A 64-bit volume serial number.
    VolumeId64(VolumeId64),
}

impl FsId {
    /// Returns the inner [`Uuid`] if this is a [`FsId::Uuid`], otherwise `None`.
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            FsId::Uuid(t) => Some(*t),
            _ => None,
        }
    }

    /// Returns the inner [`VolumeId32`] if this is a [`FsId::VolumeId32`], otherwise `None`.
    pub fn as_volumeid32(&self) -> Option<VolumeId32> {
        match self {
            FsId::VolumeId32(t) => Some(*t),
            _ => None,
        }
    }

    /// Returns the inner [`VolumeId64`] if this is a [`FsId::VolumeId64`], otherwise `None`.
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
    /// File Allocation Table 12 or [FAT12](https://en.wikipedia.org/wiki/File_Allocation_Table#FAT12)
    Fat12,
    /// File Allocation Table 16 or [FAT16](https://en.wikipedia.org/wiki/File_Allocation_Table#Final_FAT16)
    Fat16,
    /// File Allocation Table 32 or [FAT32](https://en.wikipedia.org/wiki/File_Allocation_Table#FAT32)
    Fat32,
}

#[derive(Debug)]
pub struct FsInfo {
    /// Block type, Eg: EXT4.
    fs_type: Option<FsType>,
    /// Sub block type, Eg: Filsystem is VFAT but subtype is FAT16.
    sub_type: Option<SubType>,
    /// Filesystem label, Eg: `LABEL`.
    label: Option<Label>,
    /// Filesystem identifier.
    /// Eg:
    ///     UUID: `67e55044-10b1-426f-9247-bb680e5fe0c8`
    ///     VolumeId32: `2a9d-b913`
    ///     VolumeId64: `17acf19235bcde78`
    fs_id: Option<FsId>,
    /// Sub member identifier.
    sub_member_id: Option<Uuid>,
    /// External log identifier.
    ext_log_id: Option<Uuid>,
    /// External journal identifier.
    ext_journal_id: Option<Uuid>,
    /// Filesystem version.
    version: Option<String>,
    /// Filesystem magic bytes.
    magic: Option<Vec<u8>>,
    /// Filesystem magic offset.
    magic_offset: Option<u64>,
    /// Filesystem size.
    fs_size: Option<u64>,
    /// Last fsblock/total number of fsblocks.
    fs_last_block: Option<u64>,
    /// Filesystem blocksize.
    fs_block_size: Option<u64>,
    /// Minimal block size accessible by the filesystem.
    block_size: Option<u64>,
    /// Endianness of filesystem.
    endianness: Option<Endianness>,
    /// OS used to create filesystem.
    creator: Option<String>,
}

impl FsInfo {
    pub(crate) fn empty() -> FsInfo {
        FsInfo {
            fs_type: None,
            sub_type: None,
            label: None,
            fs_id: None,
            sub_member_id: None,
            ext_log_id: None,
            ext_journal_id: None,
            version: None,
            magic: None,
            magic_offset: None,
            fs_size: None,
            fs_last_block: None,
            fs_block_size: None,
            block_size: None,
            endianness: None,
            creator: None,
        }
    }

    pub(crate) fn set_fs_type(&mut self, fs_type: FsType) {
        if cfg!(debug_assertions) {
            if self.fs_type.is_none() {
                self.fs_type = Some(fs_type);
            } else {
                panic!("`fs_type` set twice")
            }
        } else {
            self.fs_type = Some(fs_type);
        }
    }

    pub fn fs_type(&self) -> Option<FsType> {
        self.fs_type
    }

    pub(crate) fn set_sub_type(&mut self, sub_type: SubType) {
        if cfg!(debug_assertions) {
            if self.sub_type.is_none() {
                self.sub_type = Some(sub_type);
            } else {
                panic!("`sub_type` set twice")
            }
        } else {
            self.sub_type = Some(sub_type);
        }
    }

    pub fn sub_type(&self) -> Option<SubType> {
        self.sub_type
    }

    pub(crate) fn set_label(&mut self, label: Label) {
        if cfg!(debug_assertions) {
            if self.label.is_none() {
                self.label = Some(label);
            } else {
                panic!("`label` set twice")
            }
        } else {
            self.label = Some(label);
        }
    }

    pub fn label(&self) -> Option<&Label> {
        self.label.as_ref()
    }

    pub(crate) fn set_fs_id(&mut self, fs_id: FsId) {
        if cfg!(debug_assertions) {
            if self.fs_id.is_none() {
                self.fs_id = Some(fs_id);
            } else {
                panic!("`fs_id` set twice")
            }
        } else {
            self.fs_id = Some(fs_id);
        }
    }

    pub fn fs_id(&self) -> Option<FsId> {
        self.fs_id
    }

    pub(crate) fn set_sub_member_id(&mut self, sub_member_id: Uuid) {
        if cfg!(debug_assertions) {
            if self.sub_member_id.is_none() {
                self.sub_member_id = Some(sub_member_id);
            } else {
                panic!("`sub_member_id` set twice")
            }
        } else {
            self.sub_member_id = Some(sub_member_id);
        }
    }

    pub fn sub_member_id(&self) -> Option<Uuid> {
        self.sub_member_id
    }

    pub(crate) fn set_ext_log_id(&mut self, ext_log_id: Uuid) {
        if cfg!(debug_assertions) {
            if self.ext_log_id.is_none() {
                self.ext_log_id = Some(ext_log_id);
            } else {
                panic!("`ext_log_id` set twice")
            }
        } else {
            self.ext_log_id = Some(ext_log_id);
        }
    }

    pub fn ext_log_id(&self) -> Option<Uuid> {
        self.ext_log_id
    }

    pub(crate) fn set_ext_journal_id(&mut self, ext_journal_id: Uuid) {
        if cfg!(debug_assertions) {
            if self.ext_journal_id.is_none() {
                self.ext_journal_id = Some(ext_journal_id);
            } else {
                panic!("`ext_journal_id` set twice")
            }
        } else {
            self.ext_journal_id = Some(ext_journal_id);
        }
    }

    pub fn ext_journal_id(&self) -> Option<Uuid> {
        self.ext_journal_id
    }

    pub(crate) fn set_version(&mut self, version: String) {
        if cfg!(debug_assertions) {
            if self.version.is_none() {
                self.version = Some(version);
            } else {
                panic!("`version` set twice")
            }
        } else {
            self.version = Some(version);
        }
    }

    pub fn version(&self) -> Option<&String> {
        self.version.as_ref()
    }

    pub(crate) fn set_magic(&mut self, bytes: Vec<u8>, offset: u64) {
        if cfg!(debug_assertions) {
            if self.magic.is_none() {
                self.magic = Some(bytes);
            } else {
                panic!("`magic` set twice")
            }
        } else {
            self.magic = Some(bytes);
        }

        if cfg!(debug_assertions) {
            if self.magic_offset.is_none() {
                self.magic_offset = Some(offset);
            } else {
                panic!("`magic_offset` set twice")
            }
        } else {
            self.magic_offset = Some(offset);
        }
    }

    pub fn magic(&self) -> Option<(&[u8], u64)> {
        match (&self.magic, &self.magic_offset) {
            (Some(magic), Some(offset)) => Some((magic.as_slice(), *offset)),
            (None, None) => None,
            _ => unreachable!("magic and magic_offset are only ever set together via `set_magic`"),
        }
    }

    pub(crate) fn set_fs_size(&mut self, fs_size: u64) {
        if cfg!(debug_assertions) {
            if self.fs_size.is_none() {
                self.fs_size = Some(fs_size);
            } else {
                panic!("`fs_size` set twice")
            }
        } else {
            self.fs_size = Some(fs_size);
        }
    }

    pub fn fs_size(&self) -> Option<u64> {
        self.fs_size
    }

    pub(crate) fn set_fs_last_block(&mut self, fs_last_block: u64) {
        if cfg!(debug_assertions) {
            if self.fs_last_block.is_none() {
                self.fs_last_block = Some(fs_last_block);
            } else {
                panic!("`fs_last_block` set twice")
            }
        } else {
            self.fs_last_block = Some(fs_last_block);
        }
    }

    pub fn fs_last_block(&self) -> Option<u64> {
        self.fs_last_block
    }

    pub(crate) fn set_fs_block_size(&mut self, fs_block_size: u64) {
        if cfg!(debug_assertions) {
            if self.fs_block_size.is_none() {
                self.fs_block_size = Some(fs_block_size);
            } else {
                panic!("`fs_block_size` set twice")
            }
        } else {
            self.fs_block_size = Some(fs_block_size);
        }
    }

    pub fn fs_block_size(&self) -> Option<u64> {
        self.fs_block_size
    }

    pub(crate) fn set_block_size(&mut self, block_size: u64) {
        if cfg!(debug_assertions) {
            if self.block_size.is_none() {
                self.block_size = Some(block_size);
            } else {
                panic!("`block_size` set twice")
            }
        } else {
            self.block_size = Some(block_size);
        }
    }

    pub fn block_size(&self) -> Option<u64> {
        self.block_size
    }

    pub(crate) fn set_endianness(&mut self, endianness: Endianness) {
        if cfg!(debug_assertions) {
            if self.endianness.is_none() {
                self.endianness = Some(endianness);
            } else {
                panic!("`endianness` set twice")
            }
        } else {
            self.endianness = Some(endianness);
        }
    }

    pub fn endianness(&self) -> Option<Endianness> {
        self.endianness
    }

    pub(crate) fn set_creator(&mut self, creator: String) {
        if cfg!(debug_assertions) {
            if self.creator.is_none() {
                self.creator = Some(creator);
            } else {
                panic!("`creator` set twice")
            }
        } else {
            self.creator = Some(creator);
        }
    }

    pub fn creator(&self) -> Option<&String> {
        self.creator.as_ref()
    }

    #[cfg(feature = "serde")]
    pub fn filtered<'a>(&'a self, fields: Fields) -> FilteredFsInfo<'a> {
        FilteredFsInfo::new(self, fields)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for FsInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let len = self.fs_type.is_some() as usize
            + self.sub_type.is_some() as usize
            + self.label.is_some() as usize
            + self.fs_id.is_some() as usize
            + self.sub_member_id.is_some() as usize
            + self.ext_log_id.is_some() as usize
            + self.ext_journal_id.is_some() as usize
            + self.version.is_some() as usize
            + self.fs_size.is_some() as usize
            + self.fs_last_block.is_some() as usize
            + self.fs_block_size.is_some() as usize
            + self.block_size.is_some() as usize
            + self.endianness.is_some() as usize
            + self.creator.is_some() as usize
            + if self.magic.is_some() && self.magic_offset.is_some() {
                2
            } else {
                0
            };

        let mut map = serializer.serialize_map(Some(len))?;

        if let Some(v) = &self.fs_type {
            map.serialize_entry("FS_TYPE", v)?;
        }
        if let Some(v) = &self.sub_type {
            map.serialize_entry("SUB_TYPE", v)?;
        }
        if let Some(v) = &self.label {
            match v {
                Label::Utf8(utf8) => {
                    map.serialize_entry("LABEL", &utf8.to_string().trim_end_matches('\0'))?;
                }
                Label::Utf16(utf16) => {
                    map.serialize_entry("LABEL", &utf16.to_string_lossy().trim_end_matches('\0'))?
                }
            }
        }
        if let Some(v) = &self.fs_id {
            match v {
                FsId::Uuid(uuid) => map.serialize_entry("FS_ID", uuid)?,
                FsId::VolumeId32(id32) => map.serialize_entry("FS_ID", id32)?,
                FsId::VolumeId64(id64) => map.serialize_entry("FS_ID", id64)?,
            }
        }
        if let Some(v) = &self.sub_member_id {
            map.serialize_entry("SUB_MEMBER_ID", v)?;
        }
        if let Some(v) = &self.ext_log_id {
            map.serialize_entry("EXT_LOG_ID", v)?;
        }
        if let Some(v) = &self.ext_journal_id {
            map.serialize_entry("EXT_JOURNAL_ID", v)?;
        }
        if let Some(v) = &self.version {
            map.serialize_entry("VERSION", v)?;
        }
        match (&self.magic, &self.magic_offset) {
            (Some(magic), Some(offset)) => {
                map.serialize_entry("MAGIC", magic)?;
                map.serialize_entry("MAGIC_OFFSET", offset)?;
            }
            (None, None) => {}
            _ => unreachable!("magic and magic_offset are only ever set together via set_magic"),
        }
        if let Some(v) = &self.fs_size {
            map.serialize_entry("FS_SIZE", v)?;
        }
        if let Some(v) = &self.fs_last_block {
            map.serialize_entry("FS_LAST_BLOCK", v)?;
        }
        if let Some(v) = &self.fs_block_size {
            map.serialize_entry("FS_BLOCK_SIZE", v)?;
        }
        if let Some(v) = &self.block_size {
            map.serialize_entry("BLOCK_SIZE", v)?;
        }
        if let Some(v) = &self.endianness {
            map.serialize_entry("ENDIANNESS", v)?;
        }
        if let Some(v) = &self.creator {
            map.serialize_entry("CREATOR", v)?;
        }

        map.end()
    }
}

bitflags! {
    /// Filesystem types to skip checking for in probe operations.
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct FsFilter: u64 {
        #[bitflags(flag_name = "apfs")]
        const SKIP_APFS = 1 << 0;
        #[bitflags(flag_name = "cramfs")]
        const SKIP_CRAMFS = 1 << 1;
        #[bitflags(flag_name = "exfat")]
        const SKIP_EXFAT = 1 << 2;
        #[bitflags(flag_name = "jbd")]
        const SKIP_JBD = 1 << 3;
        #[bitflags(flag_name = "ext2")]
        const SKIP_EXT2 = 1 << 4;
        #[bitflags(flag_name = "ext3")]
        const SKIP_EXT3 = 1 << 5;
        #[bitflags(flag_name = "ext4")]
        const SKIP_EXT4 = 1 << 6;
        #[bitflags(flag_name = "luks1")]
        const SKIP_LUKS1 = 1 << 7;
        #[bitflags(flag_name = "luks2")]
        const SKIP_LUKS2 = 1 << 8;
        #[bitflags(flag_name = "luks_opal")]
        const SKIP_LUKS_OPAL = 1 << 9;
        #[bitflags(flag_name = "ntfs")]
        const SKIP_NTFS = 1 << 10;
        #[bitflags(flag_name = "squashfs")]
        const SKIP_SQUASHFS = 1 << 11;
        #[bitflags(flag_name = "squashfs3")]
        const SKIP_SQUASHFS3 = 1 << 12;
        #[bitflags(flag_name = "vfat")]
        const SKIP_VFAT = 1 << 13;
        #[bitflags(flag_name = "vxfs")]
        const SKIP_VXFS = 1 << 14;
        #[bitflags(flag_name = "xfs")]
        const SKIP_XFS = 1 << 15;
    }
}

#[cfg(feature = "serde")]
bitflags! {
    #[non_exhaustive]
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct Fields: u64 {
        const FS_TYPE = 1 << 0;
        const SUB_TYPE = 1 << 1;
        const LABEL = 1 << 2;
        const FS_ID = 1 << 3;
        const SUB_MEMBER_ID = 1 << 4;
        const EXT_LOG_ID = 1 << 5;
        const EXT_JOURNAL_ID = 1 << 6;
        const VERSION = 1 << 7;
        const MAGIC = 1 << 8;
        const FS_SIZE = 1 << 9;
        const FS_LAST_BLOCK = 1 << 10;
        const FS_BLOCK_SIZE = 1 << 11;
        const BLOCK_SIZE = 1 << 12;
        const ENDIANNESS = 1 << 13;
        const CREATOR = 1 << 14;
    }
}

#[cfg(feature = "serde")]
pub struct FilteredFsInfo<'a> {
    fs_info: &'a FsInfo,
    fields: Fields,
}

#[cfg(feature = "serde")]
impl<'a> FilteredFsInfo<'a> {
    fn new(fs_info: &'a FsInfo, fields: Fields) -> FilteredFsInfo<'a> {
        FilteredFsInfo { fs_info, fields }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for FilteredFsInfo<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use bitflags::Flags;
        use serde::ser::SerializeMap;

        let len = self.fields.known_bits().count_ones() as usize;

        let mut map = serializer.serialize_map(Some(len))?;

        if self.fields.contains(Fields::FS_TYPE) {
            map.serialize_entry("FS_TYPE", &self.fs_info.fs_type.expect("FS_TYPE NONE"))?;
        }
        if self.fields.contains(Fields::SUB_TYPE) {
            map.serialize_entry("SUB_TYPE", &self.fs_info.sub_type.expect("SUB_TYPE NONE"))?;
        }
        if self.fields.contains(Fields::LABEL) {
            match &self.fs_info.label.as_ref().expect("LABEL NONE") {
                Label::Utf8(utf8) => {
                    map.serialize_entry("LABEL", &utf8.to_string().trim_end_matches('\0'))?;
                }
                Label::Utf16(utf16) => {
                    map.serialize_entry("LABEL", &utf16.to_string_lossy().trim_end_matches('\0'))?
                }
            }
        }
        if self.fields.contains(Fields::FS_ID) {
            match &self.fs_info.fs_id.expect("FS_ID NONE") {
                FsId::Uuid(uuid) => map.serialize_entry("FS_ID", uuid)?,
                FsId::VolumeId32(id32) => map.serialize_entry("FS_ID", id32)?,
                FsId::VolumeId64(id64) => map.serialize_entry("FS_ID", id64)?,
            }
        }
        if self.fields.contains(Fields::SUB_MEMBER_ID) {
            map.serialize_entry(
                "SUB_MEMBER_ID",
                &self.fs_info.sub_member_id.expect("SUB_MEMBER_ID NONE"),
            )?;
        }
        if self.fields.contains(Fields::EXT_LOG_ID) {
            map.serialize_entry(
                "EXT_LOG_ID",
                &self.fs_info.ext_log_id.expect("EXT_LOG_ID NONE"),
            )?;
        }
        if self.fields.contains(Fields::EXT_JOURNAL_ID) {
            map.serialize_entry(
                "EXT_JOURNAL_ID",
                &self.fs_info.ext_journal_id.expect("EXT_JOURNAL_ID NONE"),
            )?;
        }
        if self.fields.contains(Fields::VERSION) {
            map.serialize_entry(
                "VERSION",
                &self.fs_info.version.as_ref().expect("VERSION NONE"),
            )?;
        }
        if self.fields.contains(Fields::MAGIC) {
            map.serialize_entry("MAGIC", &self.fs_info.magic.as_ref().expect("MAGIC NONE"))?;
            map.serialize_entry(
                "MAGIC_OFFSET",
                &self.fs_info.magic_offset.expect("MAGIC_OFFSET NONE"),
            )?;
        }
        if self.fields.contains(Fields::FS_SIZE) {
            map.serialize_entry("FS_SIZE", &self.fs_info.fs_size.expect("FS_SIZE NONE"))?;
        }
        if self.fields.contains(Fields::FS_LAST_BLOCK) {
            map.serialize_entry(
                "FS_LAST_BLOCK",
                &self.fs_info.fs_last_block.expect("FS_LAST_BLOCK NONE"),
            )?;
        }
        if self.fields.contains(Fields::FS_BLOCK_SIZE) {
            map.serialize_entry(
                "FS_BLOCK_SIZE",
                &self.fs_info.fs_block_size.expect("FS_BLOCK_SIZE NONE"),
            )?;
        }
        if self.fields.contains(Fields::BLOCK_SIZE) {
            map.serialize_entry(
                "BLOCK_SIZE",
                &self.fs_info.block_size.expect("BLOCK_SIZE NONE"),
            )?;
        }
        if self.fields.contains(Fields::ENDIANNESS) {
            map.serialize_entry(
                "ENDIANNESS",
                &self.fs_info.endianness.expect("ENDIANNESS NONE"),
            )?;
        }
        if self.fields.contains(Fields::CREATOR) {
            map.serialize_entry(
                "CREATOR",
                &self.fs_info.creator.as_ref().expect("CREATOR NONE"),
            )?;
        }

        map.end()
    }
}
