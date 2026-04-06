use bitflags::bitflags;
use crc_fast::{CrcParams, checksum_with_params};
use uuid::Uuid;
use zerocopy::transmute;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, Unaligned, byteorder::LittleEndian, byteorder::U16,
    byteorder::U32, byteorder::U64,
};

use crate::util::decode_utf8_lossy_from;
use crate::{
    error::{Error, ErrorKind},
    io::{BlockIo, Reader},
    probe::{BlockInfo, BlockType, Id, Magic, Tag, Usage},
    std::{fmt, mem::offset_of},
};

/*
https://www.kernel.org/doc/html/latest/filesystems/ext4/globals.html
*/

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtError {
    ProbablyLegacyExt,
    ProbablyExt4Dev,
    HeaderChecksumInvalid,
    Ext2BlockHasJournal,
    Ext3BlockMissingJournal,
    MissingExt3FeatureIncompatJournalDev,
    InvalidExt2Features,
    InvalidExt3Features,
    InvalidExt4Features,
    Ext4DetectedAsJbd,
}

impl fmt::Display for ExtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtError::ProbablyLegacyExt => write!(f, "Filesystem detected as legacy EXT"),
            ExtError::ProbablyExt4Dev => write!(f, "Filesystem detected as EXT4dev"),
            ExtError::HeaderChecksumInvalid => write!(f, "Invalid header checksum"),
            ExtError::Ext2BlockHasJournal => writeln!(f, "EXT2 does not have a journal"),
            ExtError::Ext3BlockMissingJournal => write!(f, "EXT3 requires to have a journal"),
            ExtError::MissingExt3FeatureIncompatJournalDev => {
                write!(f, "Missing EXT3 Feature Incompat Journal Dev")
            }
            ExtError::InvalidExt2Features => write!(f, "Invalid EXT2 features"),
            ExtError::InvalidExt3Features => write!(f, "Invalid EXT3 features"),
            ExtError::InvalidExt4Features => write!(f, "Invalid EXT4 features"),
            ExtError::Ext4DetectedAsJbd => write!(f, "EXT4 detected as JBD"),
        }
    }
}

impl<IO: BlockIo> From<ExtError> for Error<IO> {
    fn from(e: ExtError) -> Self {
        Self(ErrorKind::ExtError(e))
    }
}

pub const EXT_MAGICS: Option<&'static [Magic]> = Some(&[Magic {
    magic: &[0x53, 0xEF],
    len: 2,
    b_offset: 0x438,
}]);

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct Ext2SuperBlock {
    pub s_inodes_count: U32<LittleEndian>,
    pub s_blocks_count: U32<LittleEndian>,
    pub s_r_blocks_count: U32<LittleEndian>,
    pub s_free_blocks_count: U32<LittleEndian>,
    pub s_free_inodes_count: U32<LittleEndian>,
    pub s_first_data_block: U32<LittleEndian>,
    pub s_log_block_size: U32<LittleEndian>,
    s_dummy3: [U32<LittleEndian>; 7],
    pub s_magic: [u8; 2],
    pub s_state: U16<LittleEndian>,
    pub s_errors: U16<LittleEndian>,
    pub s_minor_rev_level: U16<LittleEndian>,
    pub s_lastcheck: U32<LittleEndian>,
    pub s_checkinterval: U32<LittleEndian>,
    pub s_creator_os: ExtCreator,
    pub s_rev_level: U32<LittleEndian>,
    pub s_def_resuid: U16<LittleEndian>,
    pub s_def_resgid: U16<LittleEndian>,
    pub s_first_ino: U32<LittleEndian>,
    pub s_inode_size: U16<LittleEndian>,
    pub s_block_group_nr: U16<LittleEndian>,
    pub s_feature_compat: U32<LittleEndian>,
    pub s_feature_incompat: U32<LittleEndian>,
    pub s_feature_ro_compat: U32<LittleEndian>,
    pub s_uuid: [u8; 16],
    pub s_volume_name: [u8; 16],
    pub s_last_mounted: [u8; 64],
    pub s_algorithm_usage_bitmap: U32<LittleEndian>,
    pub s_prealloc_blocks: u8,
    pub s_prealloc_dir_blocks: u8,
    pub s_reserved_gdt_blocks: U16<LittleEndian>,
    pub s_journal_uuid: [u8; 16],
    pub s_journal_inum: U32<LittleEndian>,
    pub s_journal_dev: U32<LittleEndian>,
    pub s_last_orphan: U32<LittleEndian>,
    pub s_hash_seed: [U32<LittleEndian>; 4],
    pub s_def_hash_version: u8,
    pub s_jnl_backup_type: u8,
    pub s_reserved_word_pad: U16<LittleEndian>,
    pub s_default_mount_opts: U32<LittleEndian>,
    pub s_first_meta_bg: U32<LittleEndian>,
    pub s_mkfs_time: U32<LittleEndian>,
    pub s_jnl_blocks: [U32<LittleEndian>; 17],
    pub s_blocks_count_hi: U32<LittleEndian>,
    pub s_r_blocks_count_hi: U32<LittleEndian>,
    pub s_free_blocks_hi: U32<LittleEndian>,
    pub s_min_extra_isize: U16<LittleEndian>,
    pub s_want_extra_isize: U16<LittleEndian>,
    pub s_flags: U32<LittleEndian>,
    pub s_raid_stride: U16<LittleEndian>,
    pub s_mmp_interval: U16<LittleEndian>,
    pub s_mmp_block: U64<LittleEndian>,
    pub s_raid_stripe_width: U32<LittleEndian>,
    s_reserved: [U32<LittleEndian>; 162],
    pub s_checksum: U32<LittleEndian>,
}

impl Ext2SuperBlock {
    /*
    fn ext_state(
            &self
        ) -> ExtState
    {
        ExtState::from_bits_truncate(u16::from(self.s_state))
    }

    fn ext_errors(
            &self
        ) -> ExtErrors
    {
        ExtErrors::from_bits_truncate(u16::from(self.s_errors))
    }
    */

    fn feature_compat(&self) -> ExtFeatureCompat {
        ExtFeatureCompat::from_bits_truncate(u32::from(self.s_feature_compat))
    }

    fn feature_incompat(&self) -> ExtFeatureIncompat {
        ExtFeatureIncompat::from_bits_truncate(u32::from(self.s_feature_incompat))
    }

    fn feature_rocompat(&self) -> ExtFeatureRoCompat {
        ExtFeatureRoCompat::from_bits_truncate(u32::from(self.s_feature_ro_compat))
    }

    fn ext_flags(&self) -> ExtFlags {
        ExtFlags::from_bits_truncate(u32::from(self.s_flags))
    }
}

bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct ExtState: u16 {
        const CleanlyUmounted = 0x0001;
        const ErrorsDetected = 0x0002;
        const OrphansbeingRecovered = 0x0004;
    }

    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct ExtErrors: u16 {
        const Continue = 1;
        const RemountRO = 2;
        const Panic = 3;
    }

    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct ExtFeatureCompat: u32 {
        const EXT3_FEATURE_COMPAT_HAS_JOURNAL = 0x0004;
    }

    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct ExtFeatureIncompat: u32 {
        const EXT2_FEATURE_INCOMPAT_FILETYPE         = 0x0002;
        const EXT3_FEATURE_INCOMPAT_RECOVER          = 0x0004;
        const EXT3_FEATURE_INCOMPAT_JOURNAL_DEV      = 0x0008;
        const EXT2_FEATURE_INCOMPAT_META_BG          = 0x0010;
        const EXT4_FEATURE_INCOMPAT_EXTENTS          = 0x0040;
        const EXT4_FEATURE_INCOMPAT_64BIT            = 0x0080;
        const EXT4_FEATURE_INCOMPAT_MMP              = 0x0100;
        const EXT4_FEATURE_INCOMPAT_FLEX_BG          = 0x0200;
    }

    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct ExtFeatureRoCompat: u32 {
        const EXT2_FEATURE_RO_COMPAT_SPARSE_SUPER     = 0x0001;
        const EXT2_FEATURE_RO_COMPAT_LARGE_FILE       = 0x0002;
        const EXT2_FEATURE_RO_COMPAT_BTREE_DIR        = 0x0004;
        const EXT4_FEATURE_RO_COMPAT_HUGE_FILE        = 0x0008;
        const EXT4_FEATURE_RO_COMPAT_GDT_CSUM         = 0x0010;
        const EXT4_FEATURE_RO_COMPAT_DIR_NLINK        = 0x0020;
        const EXT4_FEATURE_RO_COMPAT_EXTRA_ISIZE      = 0x0040;
        const EXT4_FEATURE_RO_COMPAT_METADATA_CSUM    = 0x0400;
    }

    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct ExtFlags: u32 {
        const EXT2_FLAGS_TEST_FILESYS = 0x0004;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct ExtCreator(U32<LittleEndian>);

impl std::fmt::Display for ExtCreator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match u32::from(self.0) {
            0 => write!(f, "Linux"),
            1 => write!(f, "Hurd"),
            2 => write!(f, "Masix"),
            3 => write!(f, "FreeBSD"),
            4 => write!(f, "Lites"),
            _ => write!(f, "Unknown"),
        }
    }
}

// This is abit janky but works without nightly rust
const EXT2_FEATURE_INCOMPAT_UNSUPPORTED: ExtFeatureIncompat =
    ExtFeatureIncompat::from_bits_truncate(
        !(ExtFeatureIncompat::EXT2_FEATURE_INCOMPAT_FILETYPE.bits()
            | ExtFeatureIncompat::EXT2_FEATURE_INCOMPAT_META_BG.bits()),
    );

const EXT2_FEATURE_RO_COMPAT_UNSUPPORTED: ExtFeatureRoCompat =
    ExtFeatureRoCompat::from_bits_truncate(
        !(ExtFeatureRoCompat::EXT2_FEATURE_RO_COMPAT_SPARSE_SUPER.bits()
            | ExtFeatureRoCompat::EXT2_FEATURE_RO_COMPAT_LARGE_FILE.bits()
            | ExtFeatureRoCompat::EXT2_FEATURE_RO_COMPAT_BTREE_DIR.bits()),
    );

const EXT3_FEATURE_INCOMPAT_UNSUPPORTED: ExtFeatureIncompat =
    ExtFeatureIncompat::from_bits_truncate(
        !(ExtFeatureIncompat::EXT2_FEATURE_INCOMPAT_FILETYPE.bits()
            | ExtFeatureIncompat::EXT3_FEATURE_INCOMPAT_RECOVER.bits()),
    );

const EXT3_FEATURE_RO_COMPAT_UNSUPPORTED: ExtFeatureRoCompat =
    ExtFeatureRoCompat::from_bits_truncate(
        !(ExtFeatureRoCompat::EXT2_FEATURE_RO_COMPAT_SPARSE_SUPER.bits()
            | ExtFeatureRoCompat::EXT2_FEATURE_RO_COMPAT_LARGE_FILE.bits()
            | ExtFeatureRoCompat::EXT2_FEATURE_RO_COMPAT_BTREE_DIR.bits()),
    );

/*
 * reads superblock and returns:
 *	fc = feature_compat
 *	fi = feature_incompat
 *	frc = feature_ro_compat
 */

fn ext_checksum(es: Ext2SuperBlock) -> Result<(), ExtError> {
    let ro_compat = es.feature_rocompat();

    if ro_compat.contains(ExtFeatureRoCompat::EXT4_FEATURE_RO_COMPAT_METADATA_CSUM) {
        // Seems EXT checksum does not XOR out the final value
        let crc32c = CrcParams::new(
            "CRC-32/ExtCrc32c",
            32,
            0x1EDC6F41,
            0xffffffff,
            true,
            0,
            0xe3069283,
        );

        let calc_sum = checksum_with_params(
            crc32c,
            &es.as_bytes()[..offset_of!(Ext2SuperBlock, s_checksum)],
        );
        let sum = u64::from(es.s_checksum);

        if sum != calc_sum {
            return Err(ExtError::HeaderChecksumInvalid);
        };
    } else if u32::from(es.s_log_block_size) >= 256 {
        return Err(ExtError::ProbablyLegacyExt);
    }

    return Ok(());
}

#[allow(clippy::type_complexity)]
fn ext_get_info(
    es: Ext2SuperBlock,
) -> Result<
    (
        Option<String>,
        Id,
        Option<Id>,
        String,
        u64,
        u64,
        u64,
        String,
    ),
    ExtError,
> {
    let fc = es.feature_compat();
    let fi = es.feature_incompat();

    let label: Option<String> = if es.s_volume_name[0] != 0 {
        Some(decode_utf8_lossy_from(&es.s_volume_name))
    } else {
        None
    };

    let uuid = Id::Uuid(Uuid::from_bytes(es.s_uuid));

    let journal_uuid: Option<Id> = if fc.contains(ExtFeatureCompat::EXT3_FEATURE_COMPAT_HAS_JOURNAL)
    {
        if es.s_journal_uuid == [0; 16] {
            None //Journal is internal to the filesystem   
        } else {
            Some(Id::Uuid(Uuid::from_bytes(es.s_journal_uuid)))
        }
    } else {
        None
    };

    let version =
        u32::from(es.s_rev_level).to_string() + "." + &u32::from(es.s_minor_rev_level).to_string();

    let log_block_size = u32::from(es.s_log_block_size);

    let block_size: u64 = if log_block_size < 32 {
        u64::from(1024u32 << log_block_size)
    } else {
        0
    };

    let fslastblock: u64 = u64::from(u32::from(es.s_blocks_count))
        | if fi.contains(ExtFeatureIncompat::EXT4_FEATURE_INCOMPAT_64BIT) {
            (u64::from(u32::from(es.s_blocks_count_hi))) << 32
        } else {
            0
        };

    let fs_size: u64 = block_size * u32::from(es.s_blocks_count) as u64;

    let creator = es.s_creator_os;

    Ok((
        label,
        uuid,
        journal_uuid,
        version,
        block_size,
        fslastblock,
        fs_size,
        creator.to_string(),
    ))
}

pub fn probe_jbd<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    magic: Magic,
) -> Result<BlockInfo, Error<IO>> {
    if magic.is_empty() {
        return Err(ErrorKind::MagicCannotBeEmpty.into());
    }

    let buf: [u8; size_of::<Ext2SuperBlock>()] = reader
        .read_exact_at::<{ size_of::<Ext2SuperBlock>() }>(offset + 1024)
        .map_err(Error::<IO>::io)?;

    let es: Ext2SuperBlock = transmute!(buf);

    let fi = es.feature_incompat();

    if !fi.contains(ExtFeatureIncompat::EXT3_FEATURE_INCOMPAT_JOURNAL_DEV) {
        return Err(ExtError::MissingExt3FeatureIncompatJournalDev.into());
    }

    let (label, uuid, journal_uuid, version, block_size, fs_last_block, fs_size, creator) =
        ext_get_info(es)?;

    let mut info = BlockInfo::new();

    info.set(Tag::FsType(BlockType::Jbd));
    if let Some(l) = label {
        info.set(Tag::Label(l));
    }
    info.set(Tag::Id(uuid));
    if let Some(id) = journal_uuid {
        info.set(Tag::ExtJournalId(id));
    }
    info.set(Tag::Usage(Usage::Filesystem));
    info.set(Tag::Version(version));
    info.set(Tag::Magic(magic.magic.to_vec()));
    info.set(Tag::MagicOffset(magic.b_offset));
    info.set(Tag::FsSize(fs_size));
    info.set(Tag::FsLastBlock(fs_last_block));
    info.set(Tag::FsBlockSize(block_size));
    info.set(Tag::BlockSize(block_size));

    return Ok(info);
}

pub fn probe_ext2<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    magic: Magic,
) -> Result<BlockInfo, Error<IO>> {
    if magic.is_empty() {
        return Err(ErrorKind::MagicCannotBeEmpty.into());
    }

    let buf: [u8; size_of::<Ext2SuperBlock>()] = reader
        .read_exact_at::<{ size_of::<Ext2SuperBlock>() }>(offset + 1024)
        .map_err(Error::<IO>::io)?;

    let es: Ext2SuperBlock = transmute!(buf);

    ext_checksum(es)?;

    let fc = es.feature_compat();
    let fi = es.feature_incompat();
    let frc = es.feature_rocompat();

    if fc.contains(ExtFeatureCompat::EXT3_FEATURE_COMPAT_HAS_JOURNAL) {
        return Err(ExtError::Ext2BlockHasJournal.into());
    };

    if frc.intersects(EXT2_FEATURE_RO_COMPAT_UNSUPPORTED)
        || fi.intersects(EXT2_FEATURE_INCOMPAT_UNSUPPORTED)
    {
        return Err(ExtError::InvalidExt2Features.into());
    }

    let (label, uuid, journal_uuid, version, block_size, fs_last_block, fs_size, creator) =
        ext_get_info(es)?;

    let mut info = BlockInfo::new();

    info.set(Tag::FsType(BlockType::Ext2));
    if let Some(l) = label {
        info.set(Tag::Label(l));
    }
    info.set(Tag::Id(uuid));
    if let Some(id) = journal_uuid {
        info.set(Tag::ExtJournalId(id));
    }
    info.set(Tag::Usage(Usage::Filesystem));
    info.set(Tag::Version(version));
    info.set(Tag::Magic(magic.magic.to_vec()));
    info.set(Tag::MagicOffset(magic.b_offset));
    info.set(Tag::FsSize(fs_size));
    info.set(Tag::FsLastBlock(fs_last_block));
    info.set(Tag::FsBlockSize(block_size));
    info.set(Tag::BlockSize(block_size));

    return Ok(info);
}

pub fn probe_ext3<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    magic: Magic,
) -> Result<BlockInfo, Error<IO>> {
    if magic.is_empty() {
        return Err(ErrorKind::MagicCannotBeEmpty.into());
    }

    let buf: [u8; size_of::<Ext2SuperBlock>()] = reader
        .read_exact_at::<{ size_of::<Ext2SuperBlock>() }>(offset + 1024)
        .map_err(Error::io)?;

    let es: Ext2SuperBlock = transmute!(buf);

    ext_checksum(es)?;

    let fc = es.feature_compat();
    let fi = es.feature_incompat();
    let frc = es.feature_rocompat();

    if !fc.contains(ExtFeatureCompat::EXT3_FEATURE_COMPAT_HAS_JOURNAL) {
        return Err(ExtError::Ext3BlockMissingJournal.into());
    };

    if frc.intersects(EXT3_FEATURE_RO_COMPAT_UNSUPPORTED)
        || fi.intersects(EXT3_FEATURE_INCOMPAT_UNSUPPORTED)
    {
        return Err(ExtError::InvalidExt3Features.into());
    }

    let (label, uuid, journal_uuid, version, block_size, fs_last_block, fs_size, creator) =
        ext_get_info(es)?;

    let mut info = BlockInfo::new();

    info.set(Tag::FsType(BlockType::Ext3));
    if let Some(l) = label {
        info.set(Tag::Label(l));
    }
    info.set(Tag::Id(uuid));
    if let Some(id) = journal_uuid {
        info.set(Tag::ExtJournalId(id));
    }
    info.set(Tag::Usage(Usage::Filesystem));
    info.set(Tag::Version(version));
    info.set(Tag::Magic(magic.magic.to_vec()));
    info.set(Tag::MagicOffset(magic.b_offset));
    info.set(Tag::FsSize(fs_size));
    info.set(Tag::FsLastBlock(fs_last_block));
    info.set(Tag::FsBlockSize(block_size));
    info.set(Tag::BlockSize(block_size));

    return Ok(info);
}

pub fn probe_ext4<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    magic: Magic,
) -> Result<BlockInfo, Error<IO>> {
    if magic.is_empty() {
        return Err(ErrorKind::MagicCannotBeEmpty.into());
    }

    let buf: [u8; size_of::<Ext2SuperBlock>()] = reader
        .read_exact_at::<{ size_of::<Ext2SuperBlock>() }>(offset + 1024)
        .map_err(Error::io)?;

    let es: Ext2SuperBlock = transmute!(buf);

    ext_checksum(es)?;

    let fi = es.feature_incompat();
    let frc = es.feature_rocompat();
    let flags = es.ext_flags();

    if fi.contains(ExtFeatureIncompat::EXT3_FEATURE_INCOMPAT_JOURNAL_DEV) {
        return Err(ExtError::Ext4DetectedAsJbd.into());
    }

    if !frc.intersects(EXT3_FEATURE_RO_COMPAT_UNSUPPORTED)
        && !fi.intersects(EXT3_FEATURE_INCOMPAT_UNSUPPORTED)
    {
        return Err(ExtError::InvalidExt4Features.into());
    }

    if flags.contains(ExtFlags::EXT2_FLAGS_TEST_FILESYS) {
        return Err(ExtError::ProbablyExt4Dev.into());
    }

    let (label, uuid, journal_uuid, version, block_size, fs_last_block, fs_size, creator) =
        ext_get_info(es)?;

    let mut info = BlockInfo::new();

    info.set(Tag::FsType(BlockType::Ext4));
    if let Some(l) = label {
        info.set(Tag::Label(l));
    }
    info.set(Tag::Id(uuid));
    if let Some(id) = journal_uuid {
        info.set(Tag::ExtJournalId(id));
    }
    info.set(Tag::Usage(Usage::Filesystem));
    info.set(Tag::Version(version));
    info.set(Tag::Magic(magic.magic.to_vec()));
    info.set(Tag::MagicOffset(magic.b_offset));
    info.set(Tag::FsSize(fs_size));
    info.set(Tag::FsLastBlock(fs_last_block));
    info.set(Tag::FsBlockSize(block_size));
    info.set(Tag::BlockSize(block_size));

    return Ok(info);
}

//fn probe_ext4dev(
//        probe: &mut BlockidProbe,
//        magic: BlockidMagic
//    ) -> Result<Option<ProbeResult>, Box<dyn std::error::Error>>
//{
//    Ok(None)
//}
