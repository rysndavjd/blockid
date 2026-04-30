use bitflags::bitflags;
use uuid::Uuid;
use zerocopy::transmute_ref;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, Unaligned, byteorder::LittleEndian, byteorder::U16,
    byteorder::U32, byteorder::U64,
};

use crate::util::decode_utf8_lossy_from;
use crate::{
    error::Error,
    io::{BlockIo, Reader},
    probe::{BlockInfo, BlockTag, BlockType, Id, Magic, Usage},
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
        Error::Ext(e)
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
    pub s_blocks_count_lo: U32<LittleEndian>,
    pub s_r_blocks_count_lo: U32<LittleEndian>,
    pub s_free_blocks_count_lo: U32<LittleEndian>,
    pub s_free_inodes_count: U32<LittleEndian>,
    pub s_first_data_block: U32<LittleEndian>,
    pub s_log_block_size: U32<LittleEndian>,
    pub s_log_cluster_size: U32<LittleEndian>,
    pub s_blocks_per_group: U32<LittleEndian>,
    pub s_clusters_per_group: U32<LittleEndian>,
    pub s_inodes_per_group: U32<LittleEndian>,
    pub s_mtime: U32<LittleEndian>,
    pub s_wtime: U32<LittleEndian>,
    pub s_mnt_count: U16<LittleEndian>,
    pub s_max_mnt_count: U16<LittleEndian>,
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
    /*
     * These fields are for EXT4_DYNAMIC_REV superblocks only.
     *
     * Note: the difference between the compatible feature set and
     * the incompatible feature set is that if there is a bit set
     * in the incompatible feature set that the kernel doesn't
     * know about, it should refuse to mount the filesystem.
     *
     * e2fsck's requirements are more strict; if it doesn't know
     * about a feature in either the compatible or incompatible
     * feature set, it must abort and not try to meddle with
     * things it doesn't understand...
     */
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
    /*
     * Performance hints.  Directory preallocation should only
     * happen if the EXT4_FEATURE_COMPAT_DIR_PREALLOC flag is on.
     */
    pub s_prealloc_blocks: u8,
    pub s_prealloc_dir_blocks: u8,
    pub s_reserved_gdt_blocks: U16<LittleEndian>,
    /*
     * Journaling support valid if EXT4_FEATURE_COMPAT_HAS_JOURNAL set.
     */
    pub s_journal_uuid: [u8; 16],
    pub s_journal_inum: U32<LittleEndian>,
    pub s_journal_dev: U32<LittleEndian>,
    pub s_last_orphan: U32<LittleEndian>,
    pub s_hash_seed: [U32<LittleEndian>; 4],
    pub s_def_hash_version: u8,
    pub s_jnl_backup_type: u8,
    pub s_desc_size: U16<LittleEndian>,
    pub s_default_mount_opts: U32<LittleEndian>,
    pub s_first_meta_bg: U32<LittleEndian>,
    pub s_mkfs_time: U32<LittleEndian>,
    pub s_jnl_blocks: [U32<LittleEndian>; 17],
    /* 64bit support valid if EXT4_FEATURE_INCOMPAT_64BIT */
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
    pub s_log_groups_per_flex: u8,
    pub s_checksum_type: u8,
    pub s_encryption_level: u8,
    pub s_reserved_pad: u8,
    pub s_kbytes_written: U64<LittleEndian>,
    pub s_snapshot_inum: U32<LittleEndian>,
    pub s_snapshot_id: U32<LittleEndian>,
    pub s_snapshot_r_blocks_count: U64<LittleEndian>,

    pub s_snapshot_list: U32<LittleEndian>,

    pub s_error_count: U32<LittleEndian>,
    pub s_first_error_time: U32<LittleEndian>,
    pub s_first_error_ino: U32<LittleEndian>,
    pub s_first_error_block: U64<LittleEndian>,
    pub s_first_error_func: [u8; 32],
    pub s_first_error_line: U32<LittleEndian>,
    pub s_last_error_time: U32<LittleEndian>,
    pub s_last_error_ino: U32<LittleEndian>,
    pub s_last_error_line: U32<LittleEndian>,
    pub s_last_error_block: U64<LittleEndian>,
    pub s_last_error_func: [u8; 32],
    pub s_mount_opts: [u8; 64],
    pub s_usr_quota_inum: U32<LittleEndian>,
    pub s_grp_quota_inum: U32<LittleEndian>,
    pub s_overhead_clusters: U32<LittleEndian>,
    pub s_backup_bgs: [U32<LittleEndian>; 2],
    pub s_encrypt_algos: [u8; 4],
    pub s_encrypt_pw_salt: [u8; 16],
    pub s_lpf_ino: U32<LittleEndian>,
    pub s_prj_quota_inum: U32<LittleEndian>,
    pub s_checksum_seed: U32<LittleEndian>,
    pub s_wtime_hi: u8,
    pub s_mtime_hi: u8,
    pub s_mkfs_time_hi: u8,
    pub s_lastcheck_hi: u8,
    pub s_first_error_time_hi: u8,
    pub s_last_error_time_hi: u8,
    pub s_first_error_errcode: u8,
    pub s_last_error_errcode: u8,
    pub s_encoding: U16<LittleEndian>,
    pub s_encoding_flags: U16<LittleEndian>,
    pub s_orphan_file_inum: U32<LittleEndian>,
    pub s_def_resuid_hi: U16<LittleEndian>,
    pub s_def_resgid_hi: U16<LittleEndian>,
    s_reserved: [U32<LittleEndian>; 93],
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

    fn get_block_count(&self) -> u64 {
        u64::from(self.s_blocks_count_lo)
            | if self
                .feature_incompat()
                .contains(ExtFeatureIncompat::SixtyFourBIT)
            {
                u64::from(self.s_blocks_count_hi) << 32
            } else {
                0
            }
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
        const HAS_JOURNAL = 0x0004;
        const SPARSE_SUPER2 = 0x200;
    }

    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct ExtFeatureIncompat: u32 {
        const FILETYPE         = 0x0002;
        const RECOVER          = 0x0004;
        const JOURNAL_DEV      = 0x0008;
        const META_BG          = 0x0010;
        const EXTENTS          = 0x0040;
        const SixtyFourBIT     = 0x0080;
        const MMP              = 0x0100;
        const FLEX_BG          = 0x0200;
    }

    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct ExtFeatureRoCompat: u32 {
        const SPARSE_SUPER     = 0x0001;
        const LARGE_FILE       = 0x0002;
        const BTREE_DIR        = 0x0004;
        const HUGE_FILE        = 0x0008;
        const GDT_CSUM         = 0x0010;
        const DIR_NLINK        = 0x0020;
        const EXTRA_ISIZE      = 0x0040;
        const BIGALLOC         = 0x0200;
        const METADATA_CSUM    = 0x0400;
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
        !(ExtFeatureIncompat::FILETYPE.bits() | ExtFeatureIncompat::META_BG.bits()),
    );

const EXT2_FEATURE_RO_COMPAT_UNSUPPORTED: ExtFeatureRoCompat =
    ExtFeatureRoCompat::from_bits_truncate(
        !(ExtFeatureRoCompat::SPARSE_SUPER.bits()
            | ExtFeatureRoCompat::LARGE_FILE.bits()
            | ExtFeatureRoCompat::BTREE_DIR.bits()),
    );

const EXT3_FEATURE_INCOMPAT_UNSUPPORTED: ExtFeatureIncompat =
    ExtFeatureIncompat::from_bits_truncate(
        !(ExtFeatureIncompat::FILETYPE.bits() | ExtFeatureIncompat::RECOVER.bits()),
    );

const EXT3_FEATURE_RO_COMPAT_UNSUPPORTED: ExtFeatureRoCompat =
    ExtFeatureRoCompat::from_bits_truncate(
        !(ExtFeatureRoCompat::SPARSE_SUPER.bits()
            | ExtFeatureRoCompat::LARGE_FILE.bits()
            | ExtFeatureRoCompat::BTREE_DIR.bits()),
    );

/*
 * reads superblock and returns:
 *	fc = feature_compat
 *	fi = feature_incompat
 *	frc = feature_ro_compat
 */

fn ext_checksum(es: &Ext2SuperBlock) -> Result<(), ExtError> {
    let ro_compat = es.feature_rocompat();

    if ro_compat.contains(ExtFeatureRoCompat::METADATA_CSUM) {
        #[cfg(feature = "std")]
        {
            use crc_fast::{CrcParams, checksum_with_params};

            let crc32c = CrcParams::new("EXT_CRC", 32, 0x1EDC6F41, 0xffffffff, true, 0, 0xe3069283);

            let calc_sum = checksum_with_params(
                crc32c,
                &es.as_bytes()[..offset_of!(Ext2SuperBlock, s_checksum)],
            );
            let sum = u64::from(es.s_checksum);

            if sum != calc_sum {
                return Err(ExtError::HeaderChecksumInvalid);
            };
        }

        #[cfg(not(feature = "std"))]
        {
            use crc::Algorithm;

            const EXT_CRC: Algorithm<u32> = Algorithm {
                width: 32,
                poly: 0x1edc6f41,
                init: 0xffffffff,
                refin: true,
                refout: true,
                xorout: 0,
                check: 0xe3069283,
                residue: 0xb798b438,
            };

            let crc = crc::Crc::<u32>::new(&EXT_CRC);
            let mut digest = crc.digest();

            digest.update(&es.as_bytes()[..offset_of!(Ext2SuperBlock, s_checksum)]);

            if es.s_checksum.get() != digest.finalize() {
                return Err(ExtError::HeaderChecksumInvalid);
            }
        }
    } else if u32::from(es.s_log_block_size) >= 256 {
        return Err(ExtError::ProbablyLegacyExt);
    }

    return Ok(());
}

#[allow(clippy::type_complexity)]
fn ext_get_info(
    es: &Ext2SuperBlock,
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

    let label: Option<String> = if es.s_volume_name[0] != 0 {
        Some(decode_utf8_lossy_from(&es.s_volume_name))
    } else {
        None
    };

    let uuid = Id::Uuid(Uuid::from_bytes(es.s_uuid));

    let journal_uuid: Option<Id> = if fc.contains(ExtFeatureCompat::HAS_JOURNAL) {
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

    let fs_size: u64 = block_size * es.get_block_count();

    let fslastblock: u64 = es.get_block_count();

    Ok((
        label,
        uuid,
        journal_uuid,
        version,
        block_size,
        fslastblock,
        fs_size,
        es.s_creator_os.to_string(),
    ))
}

pub fn probe_jbd<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    magic: Magic,
) -> Result<BlockInfo, Error<IO>> {
    let buf: [u8; size_of::<Ext2SuperBlock>()] = reader
        .read_exact_at(offset + 1024)
        .map_err(Error::<IO>::io)?;

    let es: &Ext2SuperBlock = transmute_ref!(&buf);

    let fi = es.feature_incompat();

    if !fi.contains(ExtFeatureIncompat::JOURNAL_DEV) {
        return Err(ExtError::MissingExt3FeatureIncompatJournalDev.into());
    }

    let (label, uuid, journal_uuid, version, block_size, fs_last_block, fs_size, creator) =
        ext_get_info(es)?;

    let mut info = BlockInfo::new();

    info.set(BlockTag::BlockType(BlockType::Jbd));
    if let Some(l) = label {
        info.set(BlockTag::Label(l));
    }
    info.set(BlockTag::Id(uuid));
    if let Some(id) = journal_uuid {
        info.set(BlockTag::ExtJournalId(id));
    }
    info.set(BlockTag::Usage(Usage::Filesystem));
    info.set(BlockTag::Version(version));
    info.set(BlockTag::Magic(magic.magic.to_vec()));
    info.set(BlockTag::MagicOffset(magic.b_offset));
    info.set(BlockTag::FsSize(fs_size));
    info.set(BlockTag::FsLastBlock(fs_last_block));
    info.set(BlockTag::FsBlockSize(block_size));
    info.set(BlockTag::BlockSize(block_size));
    info.set(BlockTag::Creator(creator));

    return Ok(info);
}

pub fn probe_ext2<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    magic: Magic,
) -> Result<BlockInfo, Error<IO>> {
    let buf: [u8; size_of::<Ext2SuperBlock>()] = reader
        .read_exact_at(offset + 1024)
        .map_err(Error::<IO>::io)?;

    let es: &Ext2SuperBlock = transmute_ref!(&buf);

    ext_checksum(es)?;

    let fc = es.feature_compat();
    let fi = es.feature_incompat();
    let frc = es.feature_rocompat();

    if fc.contains(ExtFeatureCompat::HAS_JOURNAL) {
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

    info.set(BlockTag::BlockType(BlockType::Ext2));
    if let Some(l) = label {
        info.set(BlockTag::Label(l));
    }
    info.set(BlockTag::Id(uuid));
    if let Some(id) = journal_uuid {
        info.set(BlockTag::ExtJournalId(id));
    }
    info.set(BlockTag::Usage(Usage::Filesystem));
    info.set(BlockTag::Version(version));
    info.set(BlockTag::Magic(magic.magic.to_vec()));
    info.set(BlockTag::MagicOffset(magic.b_offset));
    info.set(BlockTag::FsSize(fs_size));
    info.set(BlockTag::FsLastBlock(fs_last_block));
    info.set(BlockTag::FsBlockSize(block_size));
    info.set(BlockTag::BlockSize(block_size));
    info.set(BlockTag::Creator(creator));

    return Ok(info);
}

pub fn probe_ext3<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    magic: Magic,
) -> Result<BlockInfo, Error<IO>> {
    let buf: [u8; size_of::<Ext2SuperBlock>()] =
        reader.read_exact_at(offset + 1024).map_err(Error::io)?;

    let es: &Ext2SuperBlock = transmute_ref!(&buf);

    ext_checksum(es)?;

    let fc = es.feature_compat();
    let fi = es.feature_incompat();
    let frc = es.feature_rocompat();

    if !fc.contains(ExtFeatureCompat::HAS_JOURNAL) {
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

    info.set(BlockTag::BlockType(BlockType::Ext3));
    if let Some(l) = label {
        info.set(BlockTag::Label(l));
    }
    info.set(BlockTag::Id(uuid));
    if let Some(id) = journal_uuid {
        info.set(BlockTag::ExtJournalId(id));
    }
    info.set(BlockTag::Usage(Usage::Filesystem));
    info.set(BlockTag::Version(version));
    info.set(BlockTag::Magic(magic.magic.to_vec()));
    info.set(BlockTag::MagicOffset(magic.b_offset));
    info.set(BlockTag::FsSize(fs_size));
    info.set(BlockTag::FsLastBlock(fs_last_block));
    info.set(BlockTag::FsBlockSize(block_size));
    info.set(BlockTag::BlockSize(block_size));
    info.set(BlockTag::Creator(creator));

    return Ok(info);
}

pub fn probe_ext4<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    magic: Magic,
) -> Result<BlockInfo, Error<IO>> {
    let buf: [u8; size_of::<Ext2SuperBlock>()] =
        reader.read_exact_at(offset + 1024).map_err(Error::io)?;

    let es: &Ext2SuperBlock = transmute_ref!(&buf);

    ext_checksum(es)?;

    let fi = es.feature_incompat();
    let frc = es.feature_rocompat();
    let flags = es.ext_flags();

    if fi.contains(ExtFeatureIncompat::JOURNAL_DEV) {
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

    info.set(BlockTag::BlockType(BlockType::Ext4));
    if let Some(l) = label {
        info.set(BlockTag::Label(l));
    }
    info.set(BlockTag::Id(uuid));
    if let Some(id) = journal_uuid {
        info.set(BlockTag::ExtJournalId(id));
    }
    info.set(BlockTag::Usage(Usage::Filesystem));
    info.set(BlockTag::Version(version));
    info.set(BlockTag::Magic(magic.magic.to_vec()));
    info.set(BlockTag::MagicOffset(magic.b_offset));
    info.set(BlockTag::FsSize(fs_size));
    info.set(BlockTag::FsLastBlock(fs_last_block));
    info.set(BlockTag::FsBlockSize(block_size));
    info.set(BlockTag::BlockSize(block_size));
    info.set(BlockTag::Creator(creator));

    return Ok(info);
}

//fn probe_ext4dev(
//        probe: &mut BlockidProbe,
//        magic: BlockidMagic
//    ) -> Result<Option<ProbeResult>, Box<dyn std::error::Error>>
//{
//    Ok(None)
//}
