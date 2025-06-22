use std::io;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use rustix::fs::makedev;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    read_as, FilesystemResults,
    BlockidError, BlockidIdinfo, BlockidMagic, BlockidProbe,
    BlockidUUID, BlockidVersion, FsType, ProbeResult, UsageType,
    checksum::{get_crc32c, verify_crc32c, CsumAlgorium},
    filesystems::FsError,
};

/*
https://www.kernel.org/doc/html/latest/filesystems/ext4/globals.html
*/

#[derive(Error, Debug)]
pub enum ExtError {
    #[error("I/O operation failed")]
    IoError(#[from] io::Error),
    #[error("Ext Feature Error: {0}")]
    ExtFeatureError(&'static str),
    #[error("Not an Ext superblock: {0}")]
    UnknownFilesystem(&'static str),
    #[error("Crc32c Checksum failed, expected: \"{expected:X}\" and got: \"{got:X})\"")]
    ChecksumError {
        expected: CsumAlgorium,
        got: CsumAlgorium,
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

const EXT_MAGIC: [u8; 2] = [0x53, 0xEF];
const EXT_OFFSET: u64 = 0x38;

pub const JBD_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("jbd"),
    usage: Some(UsageType::Other("jbd")),
    probe_fn: |probe, magic| {
        probe_jbd(probe, magic)
        .map_err(FsError::from)
        .map_err(BlockidError::from)
    },
    minsz: None,
    magics: &[
        BlockidMagic {
            magic: &[0x53, 0xEF],
            len: 2,
            b_offset: 0x38,
        },
    ]
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
    magics: &[
        BlockidMagic {
            magic: &EXT_MAGIC,
            len: 2,
            b_offset: EXT_OFFSET,
        },
    ]
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
    magics: &[
        BlockidMagic {
            magic: &EXT_MAGIC,
            len: 2,
            b_offset: EXT_OFFSET,
        },
    ]
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
    magics: &[
        BlockidMagic {
            magic: &EXT_MAGIC,
            len: 2,
            b_offset: EXT_OFFSET,
        },
    ]
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

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Ext2SuperBlock {
    pub s_inodes_count: u32,
    pub s_blocks_count: u32,
    pub s_r_blocks_count: u32,
    pub s_free_blocks_count: u32,
    pub s_free_inodes_count: u32,
    pub s_first_data_block: u32,
    pub s_log_block_size: u32,
    s_dummy3: [u32; 7],
    pub s_magic: [u8; 2],
    pub s_state: ExtState,
    pub s_errors: ExtErrors,
    pub s_minor_rev_level: u16,
    pub s_lastcheck: u32,
    pub s_checkinterval: u32,
    pub s_creator_os: ExtCreator,
    pub s_rev_level: u32,
    pub s_def_resuid: u16,
    pub s_def_resgid: u16,
    pub s_first_ino: u32,
    pub s_inode_size: u16,
    pub s_block_group_nr: u16,
    pub s_feature_compat: ExtFeatureCompat,
    pub s_feature_incompat: ExtFeatureIncompat,
    pub s_feature_ro_compat: ExtFeatureRoCompat,
    pub s_uuid: [u8; 16],
    pub s_volume_name: [u8; 16],
    pub s_last_mounted: [u8; 64],
    pub s_algorithm_usage_bitmap: u32,
    pub s_prealloc_blocks: u8,
    pub s_prealloc_dir_blocks: u8,
    pub s_reserved_gdt_blocks: u16,
    pub s_journal_uuid: [u8; 16],
    pub s_journal_inum: u32,
    pub s_journal_dev: u32,
    pub s_last_orphan: u32,
    pub s_hash_seed: [u32; 4],
    pub s_def_hash_version: u8,
    pub s_jnl_backup_type: u8,
    pub s_reserved_word_pad: u16,
    pub s_default_mount_opts: u32,
    pub s_first_meta_bg: u32,
    pub s_mkfs_time: u32,
    pub s_jnl_blocks: [u32; 17],
    pub s_blocks_count_hi: u32,
    pub s_r_blocks_count_hi: u32,
    pub s_free_blocks_hi: u32,
    pub s_min_extra_isize: u16,
    pub s_want_extra_isize: u16,
    pub s_flags: ExtFlags,
    pub s_raid_stride: u16,
    pub s_mmp_interval: u16,
    pub s_mmp_block: u64,
    pub s_raid_stripe_width: u32,
    s_reserved: [u32; 162],
    pub s_checksum: u32,
}

bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Pod, Zeroable)]
    pub struct ExtState: u16 {
        const CleanlyUmounted = 0x0001;
        const ErrorsDetected = 0x0002;
        const OrphansbeingRecovered = 0x0004;
    }

    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Pod, Zeroable)]
    pub struct ExtErrors: u16 {
        const Continue = 1;
        const RemountRO = 2;
        const Panic = 3;
    }

    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Pod, Zeroable)]
    pub struct ExtCreator: u32 {
        const Linux = 0;
        const Hurd = 1;
        const Masix = 2;
        const FreeBSD = 3;
        const Lites = 4;
    }

    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Pod, Zeroable)]
    pub struct ExtFeatureCompat: u32 {
        const EXT3_FEATURE_COMPAT_HAS_JOURNAL = 0x0004;
    }

    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Pod, Zeroable)]
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
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Pod, Zeroable)]
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
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Pod, Zeroable)]
    pub struct ExtFlags: u32 {
        const EXT2_FLAGS_TEST_FILESYS = 0x0004;
    }
}

impl ToString for ExtCreator {
    fn to_string(&self) -> String {
        match self.bits() {
            0 => String::from("Linux"),
            1 => String::from("Hurd"),
            2 => String::from("Masix"),
            3 => String::from("FreeBSD"),
            4 => String::from("Lites"),
            _ => String::from("Unknown"),
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

// u32::from_le() == le32_to_cpu()
// .to_le() == cpu_to_le32()

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
    let ro_compat = es.s_feature_ro_compat; 
    
    if ro_compat.contains(ExtFeatureRoCompat::EXT4_FEATURE_RO_COMPAT_METADATA_CSUM) {
        let s_checksum = es.s_checksum;
        let csum = get_crc32c(&s_checksum.to_le_bytes());
    
        if !verify_crc32c(&s_checksum.to_le_bytes(), csum) {
            return Err(ExtError::ChecksumError { expected: CsumAlgorium::Crc32c(s_checksum), got: CsumAlgorium::Crc32c(csum) });
        };
    }

    return Ok(());
}

fn ext_get_info(
        es: Ext2SuperBlock,
    ) -> Result<(Option<String>, BlockidUUID, Option<BlockidUUID>, BlockidVersion, u64, u64, u64, String), ExtError>
{

    let fc = es.s_feature_compat;
    let fi = es.s_feature_incompat;
    //let frc = es.s_feature_ro_compat;

    let label: Option<String> = if es.s_volume_name[0] != 0 {
        Some(String::from_utf8_lossy(&es.s_volume_name).to_string())
    } else {
        None
    };
    
    let uuid = BlockidUUID::Standard(Uuid::from_bytes(es.s_uuid));
    
    let journal_uuid: Option<BlockidUUID> = if fc.contains(ExtFeatureCompat::EXT3_FEATURE_COMPAT_HAS_JOURNAL) {
        if es.s_journal_uuid == [0; 16] {
            None //Journal is internal to the filesystem   
        } else {
            Some(BlockidUUID::Standard(Uuid::from_bytes(es.s_journal_uuid)))
        }
    } else {
        None
    };

    let version = BlockidVersion::DevT(makedev(es.s_rev_level, es.s_minor_rev_level as u32));

    let log_block_size = u32::from_le(es.s_log_block_size);
    assert!(log_block_size < 32, "Shift too large"); 
    let block_size: u64 = (1024u32 << log_block_size).into();
    

    let fslastblock: u64 = u64::from(u32::from_le(es.s_blocks_count))
    | if fi.contains(ExtFeatureIncompat::EXT4_FEATURE_INCOMPAT_64BIT) {
        (u64::from(u32::from_le(es.s_blocks_count_hi))) << 32
    } else {
        0
    };

    let fs_size: u64 = block_size * u32::from_le(es.s_blocks_count) as u64; 

    let creator = es.s_creator_os;

    Ok((label, uuid, journal_uuid, version, block_size, fslastblock, fs_size, creator.to_string()))
}

pub fn probe_jbd(
        probe: &mut BlockidProbe, 
        _magic: BlockidMagic
    ) -> Result<ProbeResult, ExtError> 
{
    let es: Ext2SuperBlock = read_as(&mut probe.file, 1024)?;
    
    let fi = es.s_feature_incompat;

    if !fi.contains(ExtFeatureIncompat::EXT3_FEATURE_INCOMPAT_JOURNAL_DEV) {
        return Err(ExtError::ExtFeatureError("Ext missing \"EXT3_FEATURE_INCOMPAT_JOURNAL_DEV\" to be JBD fs"));
    }
    
    let (label, uuid, journal_uuid, version, block_size, fs_last_block, fs_size, creator) = ext_get_info(es)?;

    return Ok(ProbeResult::Filesystem(
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
                            block_size: Some(block_size)
                        }
                    )
                );
}

pub fn probe_ext2(
        probe: &mut BlockidProbe, 
        _magic: BlockidMagic
    ) -> Result<ProbeResult, ExtError> 
{
    let es: Ext2SuperBlock = read_as(&mut probe.file, 1024)?;

    ext_checksum(es)?;

    let fc = es.s_feature_compat;
    let fi = es.s_feature_incompat;
    let frc = es.s_feature_ro_compat;

    if fc.contains(ExtFeatureCompat::EXT3_FEATURE_COMPAT_HAS_JOURNAL) {
        return Err(ExtError::UnknownFilesystem("Block has a journal so its not ext2"))
    };

    if frc.intersects(EXT2_FEATURE_RO_COMPAT_UNSUPPORTED) ||
        fi.intersects(EXT2_FEATURE_INCOMPAT_UNSUPPORTED)
    {
        return Err(ExtError::ExtFeatureError("Block has features unsupported by ext2"))                                     
    }

    let (label, uuid, journal_uuid, version, block_size, fs_last_block, fs_size, creator) = ext_get_info(es)?;

    return Ok(ProbeResult::Filesystem(
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
                                    block_size: Some(block_size)
                                }
                            )
                        );
}

pub fn probe_ext3(
        probe: &mut BlockidProbe, 
        _magic: BlockidMagic
    ) -> Result<ProbeResult, ExtError> 
{
    let es: Ext2SuperBlock = read_as(&mut probe.file, 1024)?;

    ext_checksum(es)?;

    let fc = es.s_feature_compat;
    let fi = es.s_feature_incompat;
    let frc = es.s_feature_ro_compat;

    if !fc.contains(ExtFeatureCompat::EXT3_FEATURE_COMPAT_HAS_JOURNAL) {
        return Err(ExtError::ExtFeatureError("Block is missing journal"))
    };
    
    if frc.intersects(EXT3_FEATURE_RO_COMPAT_UNSUPPORTED) ||
        fi.intersects(EXT3_FEATURE_INCOMPAT_UNSUPPORTED)
    {
        return Err(ExtError::ExtFeatureError("Block contains features unsupported by ext3"))                                     
    }

    let (label, uuid, journal_uuid, version, block_size, fs_last_block, fs_size, creator) = ext_get_info(es)?;

    return Ok(ProbeResult::Filesystem(
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
                                    block_size: Some(block_size)
                                }
                            )
                        );
}

pub fn probe_ext4(
        probe: &mut BlockidProbe, 
        _magic: BlockidMagic
    ) -> Result<ProbeResult, ExtError> 
{
    let es: Ext2SuperBlock = read_as(&mut probe.file, 1024)?;

    ext_checksum(es)?;

    let fi = es.s_feature_incompat;
    let frc = es.s_feature_ro_compat;
    let flags = es.s_flags;

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

    return Ok(ProbeResult::Filesystem(
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
                                    block_size: Some(block_size)
                                }
                            )
                        );

}

//fn probe_ext4dev(
//        probe: &mut BlockidProbe, 
//        magic: BlockidMagic
//    ) -> Result<Option<ProbeResult>, Box<dyn std::error::Error>> 
//{
//    Ok(None)
//}