use std::io::{Error as IoError};

use bitflags::bitflags;
use zerocopy::{FromBytes, IntoBytes, Unaligned, 
    byteorder::U64, byteorder::U32, byteorder::U16, 
    byteorder::LittleEndian, Immutable};
use rustix::fs::makedev;
use uuid::Uuid;

use crate::{
    from_file, FilesystemResults,
    BlockidError, BlockidIdinfo, BlockidMagic, BlockidProbe,
    BlockidUUID, BlockidVersion, FsType, ProbeResult, UsageType,
    checksum::{get_crc32c, verify_crc32c, CsumAlgorium},
    filesystems::FsError, util::decode_utf8_lossy_from
};

/*
https://www.kernel.org/doc/html/latest/filesystems/ext4/globals.html
*/

#[derive(Debug)]
pub enum ExtError {
    IoError(IoError),
    ExtFeatureError(&'static str),
    UnknownFilesystem(&'static str),
    ChecksumError {
        expected: CsumAlgorium,
        got: CsumAlgorium,
    }
}

impl std::fmt::Display for ExtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtError::IoError(e) => write!(f, "I/O operation failed: {e}"),
            ExtError::ExtFeatureError(e) => write!(f, "{e}"),
            ExtError::UnknownFilesystem(e) => write!(f, "{e}"),
            ExtError::ChecksumError{expected, got} => write!(f, "Crc32c Checksum failed, expected: \"{expected:X}\" and got: \"{got:X})\""),
        }
    }
}

impl From<ExtError> for FsError {
    fn from(err: ExtError) -> Self {
        match err {
            ExtError::IoError(e) => FsError::IoError(e),
            ExtError::ExtFeatureError(feature) => FsError::InvalidHeader(feature),
            ExtError::UnknownFilesystem(fs) => FsError::UnknownFilesystem(fs),
            ExtError::ChecksumError { expected, got } => FsError::ChecksumError { expected, got },
        }
    }
}

impl From<IoError> for ExtError {
    fn from(err: IoError) -> Self {
        ExtError::IoError(err)
    }
}

const EXT_MAGIC: [u8; 2] = [0x53, 0xEF];
const EXT_OFFSET: u64 = 0x438;

pub const JBD_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("jbd"),
    usage: Some(UsageType::Other("jbd")),
    probe_fn: |probe, magic| {
        probe_jbd(probe, magic)
        .map_err(FsError::from)
        .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[
        BlockidMagic {
            magic: &[0x53, 0xEF],
            len: 2,
            b_offset: 0x38,
        },
    ])
};

pub const EXT2_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("ext2"),
    usage: Some(UsageType::Filesystem),
    probe_fn: |probe, magic| {
        probe_ext2(probe, magic)
        .map_err(FsError::from)
        .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[
        BlockidMagic {
            magic: &EXT_MAGIC,
            len: 2,
            b_offset: EXT_OFFSET,
        },
    ])
};

pub const EXT3_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("ext3"),
    usage: Some(UsageType::Filesystem),
    probe_fn: |probe, magic| {
        probe_ext3(probe, magic)
        .map_err(FsError::from)
        .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[
        BlockidMagic {
            magic: &EXT_MAGIC,
            len: 2,
            b_offset: EXT_OFFSET,
        },
    ])
};

pub const EXT4_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("ext4"),
    usage: Some(UsageType::Filesystem),
    probe_fn: |probe, magic| {
        probe_ext4(probe, magic)
        .map_err(FsError::from)
        .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[
        BlockidMagic {
            magic: &EXT_MAGIC,
            len: 2,
            b_offset: EXT_OFFSET,
        },
    ])
};

//pub const EXT4DEV_ID_INFO: BlockidIdinfo = BlockidIdinfo {
//    name: Some("ext4dev"),
//    usage: Some(UsageType::Filesystem),
//    probe_fn: probe_ext4dev,
//    minsz: None,
//    magics: &[
//        BlockidMagic {
//            magic: &[0x53, 0xEF],
//            len: 2,
//            b_offset: 0x38,
//        },
//    ]
//};

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

    fn feature_compat(
            &self
        ) -> ExtFeatureCompat
    {
        ExtFeatureCompat::from_bits_truncate(u32::from(self.s_feature_compat))
    }

    fn feature_incompat(
            &self
        ) -> ExtFeatureIncompat
    {
        ExtFeatureIncompat::from_bits_truncate(u32::from(self.s_feature_incompat))
    }

    fn feature_rocompat(
            &self
        ) -> ExtFeatureRoCompat
    {
        ExtFeatureRoCompat::from_bits_truncate(u32::from(self.s_feature_ro_compat))
    }

    fn ext_flags(
            &self
        ) -> ExtFlags
    {
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
        !(ExtFeatureIncompat::EXT2_FEATURE_INCOMPAT_FILETYPE.bits() | 
		ExtFeatureIncompat::EXT2_FEATURE_INCOMPAT_META_BG.bits())
    );

const EXT2_FEATURE_RO_COMPAT_UNSUPPORTED: ExtFeatureRoCompat =
    ExtFeatureRoCompat::from_bits_truncate(
        !(ExtFeatureRoCompat::EXT2_FEATURE_RO_COMPAT_SPARSE_SUPER.bits() | 
		ExtFeatureRoCompat::EXT2_FEATURE_RO_COMPAT_LARGE_FILE.bits() | 
		ExtFeatureRoCompat::EXT2_FEATURE_RO_COMPAT_BTREE_DIR.bits())
    );

const EXT3_FEATURE_INCOMPAT_UNSUPPORTED: ExtFeatureIncompat =
        ExtFeatureIncompat::from_bits_truncate(
        !(ExtFeatureIncompat::EXT2_FEATURE_INCOMPAT_FILETYPE.bits() |
        ExtFeatureIncompat::EXT3_FEATURE_INCOMPAT_RECOVER.bits())
    );

const EXT3_FEATURE_RO_COMPAT_UNSUPPORTED: ExtFeatureRoCompat =
        ExtFeatureRoCompat::from_bits_truncate(
        !(ExtFeatureRoCompat::EXT2_FEATURE_RO_COMPAT_SPARSE_SUPER.bits() | 
        ExtFeatureRoCompat::EXT2_FEATURE_RO_COMPAT_LARGE_FILE.bits() | 
        ExtFeatureRoCompat::EXT2_FEATURE_RO_COMPAT_BTREE_DIR.bits())
    );

/*
 * reads superblock and returns:
 *	fc = feature_compat
 *	fi = feature_incompat
 *	frc = feature_ro_compat
 */

fn ext_checksum(
        es: Ext2SuperBlock,
    ) -> Result<(), ExtError>
{   
    let ro_compat = es.feature_rocompat(); 
    
    if ro_compat.contains(ExtFeatureRoCompat::EXT4_FEATURE_RO_COMPAT_METADATA_CSUM) {
        let s_checksum = es.s_checksum;
        let csum = get_crc32c(&s_checksum.to_bytes());
    
        if !verify_crc32c(&s_checksum.to_bytes(), csum) {
            return Err(ExtError::ChecksumError { expected: CsumAlgorium::Crc32c(u32::from(s_checksum)), got: CsumAlgorium::Crc32c(csum) });
        };
    }

    return Ok(());
}

#[allow(clippy::type_complexity)]
fn ext_get_info(
        es: Ext2SuperBlock,
    ) -> Result<(Option<String>, BlockidUUID, Option<BlockidUUID>, BlockidVersion, u64, u64, u64, String), ExtError>
{

    let fc = es.feature_compat();
    let fi = es.feature_incompat();
    //let frc = es.s_feature_ro_compat;

    let label: Option<String> = if es.s_volume_name[0] != 0 {
        Some(decode_utf8_lossy_from(&es.s_volume_name))
    } else {
        None
    };
    
    let uuid = BlockidUUID::Uuid(Uuid::from_bytes(es.s_uuid));
    
    let journal_uuid: Option<BlockidUUID> = if fc.contains(ExtFeatureCompat::EXT3_FEATURE_COMPAT_HAS_JOURNAL) {
        if es.s_journal_uuid == [0; 16] {
            None //Journal is internal to the filesystem   
        } else {
            Some(BlockidUUID::Uuid(Uuid::from_bytes(es.s_journal_uuid)))
        }
    } else {
        None
    };

    let version = BlockidVersion::DevT(makedev(u32::from(es.s_rev_level), u32::from(es.s_minor_rev_level)));

    let log_block_size = u32::from(es.s_log_block_size);
    assert!(log_block_size < 32, "Shift too large"); 
    let block_size: u64 = (1024u32 << log_block_size).into();
    

    let fslastblock: u64 = u64::from(u32::from(es.s_blocks_count))
    | if fi.contains(ExtFeatureIncompat::EXT4_FEATURE_INCOMPAT_64BIT) {
        (u64::from(u32::from(es.s_blocks_count_hi))) << 32
    } else {
        0
    };

    let fs_size: u64 = block_size * u32::from(es.s_blocks_count) as u64; 

    let creator = es.s_creator_os;

    Ok((label, uuid, journal_uuid, version, block_size, fslastblock, fs_size, creator.to_string()))
}

pub fn probe_jbd(
        probe: &mut BlockidProbe, 
        _magic: BlockidMagic
    ) -> Result<(), ExtError> 
{
    let es: Ext2SuperBlock = from_file(&mut probe.file, probe.offset + 1024)?;
    
    let fi = es.feature_incompat();

    if !fi.contains(ExtFeatureIncompat::EXT3_FEATURE_INCOMPAT_JOURNAL_DEV) {
        return Err(ExtError::ExtFeatureError("Ext missing \"EXT3_FEATURE_INCOMPAT_JOURNAL_DEV\" to be JBD fs"));
    }
    
    let (label, uuid, journal_uuid, version, block_size, fs_last_block, fs_size, creator) = ext_get_info(es)?;

    probe.push_result(ProbeResult::Filesystem(
        FilesystemResults { fs_type: Some(FsType::Ext2), 
                            sec_type: None, 
                            label, 
                            fs_uuid: Some(uuid), 
                            log_uuid: Some(uuid), 
                            ext_journal: journal_uuid, 
                            fs_creator: Some(creator),
                            usage: Some(UsageType::Filesystem), 
                            version: Some(version), 
                            sbmagic: Some(&EXT_MAGIC), 
                            sbmagic_offset: Some(EXT_OFFSET), 
                            fs_size: Some(fs_size), 
                            fs_last_block: Some(fs_last_block),
                            fs_block_size: Some(block_size),
                            block_size: Some(block_size),
                            endianness: None,
                        }
                    )
                );
    
    return Ok(());
}

pub fn probe_ext2(
        probe: &mut BlockidProbe, 
        _magic: BlockidMagic
    ) -> Result<(), ExtError> 
{
    let es: Ext2SuperBlock = from_file(&mut probe.file, probe.offset + 1024)?;

    ext_checksum(es)?;

    let fc = es.feature_compat();
    let fi = es.feature_incompat();
    let frc = es.feature_rocompat();

    if fc.contains(ExtFeatureCompat::EXT3_FEATURE_COMPAT_HAS_JOURNAL) {
        return Err(ExtError::UnknownFilesystem("Block has a journal so its not ext2"))
    };

    if frc.intersects(EXT2_FEATURE_RO_COMPAT_UNSUPPORTED) ||
        fi.intersects(EXT2_FEATURE_INCOMPAT_UNSUPPORTED)
    {
        return Err(ExtError::ExtFeatureError("Block has features unsupported by ext2"))                                     
    }

    let (label, uuid, journal_uuid, version, block_size, fs_last_block, fs_size, creator) = ext_get_info(es)?;

    probe.push_result(ProbeResult::Filesystem(
                FilesystemResults { fs_type: Some(FsType::Ext2), 
                                    sec_type: None, 
                                    label, 
                                    fs_uuid: Some(uuid), 
                                    log_uuid: None, 
                                    ext_journal: journal_uuid, 
                                    fs_creator: Some(creator),
                                    usage: Some(UsageType::Filesystem), 
                                    version: Some(version), 
                                    sbmagic: Some(&EXT_MAGIC), 
                                    sbmagic_offset: Some(EXT_OFFSET), 
                                    fs_size: Some(fs_size), 
                                    fs_last_block: Some(fs_last_block),
                                    fs_block_size: Some(block_size),
                                    block_size: Some(block_size),
                                    endianness: None,
                                }
                            )
                        );

    return Ok(());
}

pub fn probe_ext3(
        probe: &mut BlockidProbe, 
        _magic: BlockidMagic
    ) -> Result<(), ExtError> 
{
    let es: Ext2SuperBlock = from_file(&mut probe.file, probe.offset + 1024)?;

    ext_checksum(es)?;

    let fc = es.feature_compat();
    let fi = es.feature_incompat();
    let frc = es.feature_rocompat();

    if !fc.contains(ExtFeatureCompat::EXT3_FEATURE_COMPAT_HAS_JOURNAL) {
        return Err(ExtError::ExtFeatureError("Block is missing journal"))
    };
    
    if frc.intersects(EXT3_FEATURE_RO_COMPAT_UNSUPPORTED) ||
        fi.intersects(EXT3_FEATURE_INCOMPAT_UNSUPPORTED)
    {
        return Err(ExtError::ExtFeatureError("Block contains features unsupported by ext3"))                                     
    }

    let (label, uuid, journal_uuid, version, block_size, fs_last_block, fs_size, creator) = ext_get_info(es)?;

    probe.push_result(ProbeResult::Filesystem(
                FilesystemResults { fs_type: Some(FsType::Ext3), 
                                    sec_type: None, 
                                    label, 
                                    fs_uuid: Some(uuid), 
                                    log_uuid: None, 
                                    ext_journal: journal_uuid, 
                                    fs_creator: Some(creator),
                                    usage: Some(UsageType::Filesystem), 
                                    version: Some(version), 
                                    sbmagic: Some(&EXT_MAGIC), 
                                    sbmagic_offset: Some(EXT_OFFSET), 
                                    fs_size: Some(fs_size), 
                                    fs_last_block: Some(fs_last_block),
                                    fs_block_size: Some(block_size),
                                    block_size: Some(block_size),
                                    endianness: None,
                                }
                            )
                        );
    
    return Ok(());
}

pub fn probe_ext4(
        probe: &mut BlockidProbe, 
        _magic: BlockidMagic
    ) -> Result<(), ExtError> 
{
    let es: Ext2SuperBlock = from_file(&mut probe.file, probe.offset + 1024)?;

    ext_checksum(es)?;

    let fi = es.feature_incompat();
    let frc = es.feature_rocompat();
    let flags = es.ext_flags();

    if fi.contains(ExtFeatureIncompat::EXT3_FEATURE_INCOMPAT_JOURNAL_DEV) {
        return Err(ExtError::UnknownFilesystem("Block is jbd"));
    }
        
    if !frc.intersects(EXT3_FEATURE_RO_COMPAT_UNSUPPORTED) &&
        !fi.intersects(EXT3_FEATURE_INCOMPAT_UNSUPPORTED)
    {
        return Err(ExtError::ExtFeatureError("Block missing supported features of ext4"))                                     
    }

    if flags.contains(ExtFlags::EXT2_FLAGS_TEST_FILESYS) {
        return Err(ExtError::UnknownFilesystem("Ext is ext4dev"));
    }

    let (label, uuid, journal_uuid, version, block_size, fs_last_block, fs_size, creator) = ext_get_info(es)?;

    probe.push_result(ProbeResult::Filesystem(
                FilesystemResults { fs_type: Some(FsType::Ext4), 
                                    sec_type: None, 
                                    label, 
                                    fs_uuid: Some(uuid), 
                                    log_uuid: None, 
                                    ext_journal: journal_uuid, 
                                    fs_creator: Some(creator),
                                    usage: Some(UsageType::Filesystem), 
                                    version: Some(version), 
                                    sbmagic: Some(&EXT_MAGIC), 
                                    sbmagic_offset: Some(EXT_OFFSET), 
                                    fs_size: Some(fs_size), 
                                    fs_last_block: Some(fs_last_block),
                                    fs_block_size: Some(block_size),
                                    block_size: Some(block_size),
                                    endianness: None,
                                }
                            )
                        );
    
    return Ok(());                    
}

//fn probe_ext4dev(
//        probe: &mut BlockidProbe, 
//        magic: BlockidMagic
//    ) -> Result<Option<ProbeResult>, Box<dyn std::error::Error>> 
//{
//    Ok(None)
//}