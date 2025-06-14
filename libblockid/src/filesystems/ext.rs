use std::u16;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Cursor};
use byteorder::ByteOrder;
use rustix::fs::makedev;
use uuid::Uuid;
use bytemuck::{Pod, Zeroable};
use bitflags::bitflags;

use crate::crc32c::{verify_crc32c, get_crc32c};
use crate::{read_as, FilesystemResults, FsSecType};
use crate::{FsType, BlockidMagic, BlockidIdinfo, UsageType, BlockidProbe, ProbeResult, BlockidUUID, BlockidVersion};

/*
https://www.kernel.org/doc/html/latest/filesystems/ext4/globals.html
*/

const EXT_MAGIC: [u8; 2] = [0x53, 0xEF];
const EXT_OFFSET: u64 = 0x38;

//pub const JBD_ID_INFO: BlockidIdinfo = BlockidIdinfo {
//    name: Some("jbd"),
//    usage: Some(UsageType::Other("jbd")),
//    probe_fn: probe_jbd,
//    minsz: None,
//    magics: &[
//        BlockidMagic {
//            magic: &[0x53, 0xEF],
//            len: 2,
//            b_offset: 0x38,
//        },
//    ]
//};

pub const EXT2_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("ext2"),
    usage: Some(UsageType::Filesystem),
    probe_fn: probe_ext2,
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
    probe_fn: probe_ext3,
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
    probe_fn: probe_ext4,
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
    pub s_state: u16,
    pub s_errors: u16,
    pub s_minor_rev_level: u16,
    pub s_lastcheck: u32,
    pub s_checkinterval: u32,
    pub s_creator_os: u32,
    pub s_rev_level: u32,
    pub s_def_resuid: u16,
    pub s_def_resgid: u16,
    pub s_first_ino: u32,
    pub s_inode_size: u16,
    pub s_block_group_nr: u16,
    pub s_feature_compat: u32,
    pub s_feature_incompat: u32,
    pub s_feature_ro_compat: u32,
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
    pub s_flags: u32,
    pub s_raid_stride: u16,
    pub s_mmp_interval: u16,
    pub s_mmp_block: u64,
    pub s_raid_stripe_width: u32,
    s_reserved: [u32; 162],
    pub s_checksum: u32,
}

fn has_ext_flags(flags: u32, feature: ExtFlags) -> bool {
    ExtFlags::from_bits_truncate(flags).contains(feature)
}

fn has_compat(compat: u32, feature: FeatureCompat) -> bool {
    FeatureCompat::from_bits_truncate(compat).contains(feature)
}

fn has_incompat(incompat: u32, feature: FeatureIncompat) -> bool {
    FeatureIncompat::from_bits_truncate(incompat).contains(feature)
}

fn has_rocompat(rocompat: u32, feature: FeatureRoCompat) -> bool {
    FeatureRoCompat::from_bits_truncate(rocompat).contains(feature)
}

bitflags! {
    pub struct ExtFlags: u32 {
        const EXT2_FLAGS_TEST_FILESYS = 0x0004;
    }
}

bitflags! {
    pub struct FeatureCompat: u32 {
        const EXT3_FEATURE_COMPAT_HAS_JOURNAL = 0x0004;
    }
}

bitflags! {
    pub struct FeatureIncompat: u32 {
        const EXT2_FEATURE_INCOMPAT_FILETYPE         = 0x0002;
        const EXT3_FEATURE_INCOMPAT_RECOVER          = 0x0004;
        const EXT3_FEATURE_INCOMPAT_JOURNAL_DEV      = 0x0008;
        const EXT2_FEATURE_INCOMPAT_META_BG          = 0x0010;
        const EXT4_FEATURE_INCOMPAT_EXTENTS          = 0x0040;
        const EXT4_FEATURE_INCOMPAT_64BIT            = 0x0080;
        const EXT4_FEATURE_INCOMPAT_MMP              = 0x0100;
        const EXT4_FEATURE_INCOMPAT_FLEX_BG          = 0x0200;
    }
}

bitflags! {
    pub struct FeatureRoCompat: u32 {
        const EXT2_FEATURE_RO_COMPAT_SPARSE_SUPER     = 0x0001;
        const EXT2_FEATURE_RO_COMPAT_LARGE_FILE       = 0x0002;
        const EXT2_FEATURE_RO_COMPAT_BTREE_DIR        = 0x0004;
        const EXT4_FEATURE_RO_COMPAT_HUGE_FILE        = 0x0008;
        const EXT4_FEATURE_RO_COMPAT_GDT_CSUM         = 0x0010;
        const EXT4_FEATURE_RO_COMPAT_DIR_NLINK        = 0x0020;
        const EXT4_FEATURE_RO_COMPAT_EXTRA_ISIZE      = 0x0040;
        const EXT4_FEATURE_RO_COMPAT_METADATA_CSUM    = 0x0400;
    }
}

//#[derive(Debug)]
//pub enum ExtCreator {
//    Linux, 
//    Hurd,
//    Masix,
//    FreeBSD,
//    Lites,
//}

/* Eventually I will figure a way to make these shortcuts for bitflags without using nightly rust

#define EXT2_FEATURE_RO_COMPAT_SUPP	(EXT2_FEATURE_RO_COMPAT_SPARSE_SUPER| \
					 EXT2_FEATURE_RO_COMPAT_LARGE_FILE| \
					 EXT2_FEATURE_RO_COMPAT_BTREE_DIR)
#define EXT2_FEATURE_INCOMPAT_SUPP	(EXT2_FEATURE_INCOMPAT_FILETYPE| \
					 EXT2_FEATURE_INCOMPAT_META_BG)
#define EXT2_FEATURE_INCOMPAT_UNSUPPORTED	~EXT2_FEATURE_INCOMPAT_SUPP
#define EXT2_FEATURE_RO_COMPAT_UNSUPPORTED	~EXT2_FEATURE_RO_COMPAT_SUPP

#define EXT3_FEATURE_RO_COMPAT_SUPP	(EXT2_FEATURE_RO_COMPAT_SPARSE_SUPER| \
					 EXT2_FEATURE_RO_COMPAT_LARGE_FILE| \
					 EXT2_FEATURE_RO_COMPAT_BTREE_DIR)
#define EXT3_FEATURE_INCOMPAT_SUPP	(EXT2_FEATURE_INCOMPAT_FILETYPE| \
					 EXT3_FEATURE_INCOMPAT_RECOVER| \
					 EXT2_FEATURE_INCOMPAT_META_BG)
#define EXT3_FEATURE_INCOMPAT_UNSUPPORTED	~EXT3_FEATURE_INCOMPAT_SUPP
#define EXT3_FEATURE_RO_COMPAT_UNSUPPORTED	~EXT3_FEATURE_RO_COMPAT_SUPP
*/

// u32::from_le() == le32_to_cpu()
// .to_le() == cpu_to_le32()

fn get_os_creator(
        es: Ext2SuperBlock,
    ) -> Option<&'static str>
{
    match es.s_creator_os {
        0 => Some("Linux"),
        1 => Some("Hurd"),
        2 => Some("Masix"),
        3 => Some("FreeBSD"),
        4 => Some("Lites"),
        _ => None,
    }
}

/*
 * reads superblock and returns:
 *	fc = feature_compat
 *	fi = feature_incompat
 *	frc = feature_ro_compat
 */

fn ext_get_super(
        es: Ext2SuperBlock,
    ) -> Result<(u32, u32, u32), Box<dyn std::error::Error>>
{   
    if has_rocompat(es.s_feature_ro_compat, FeatureRoCompat::EXT4_FEATURE_RO_COMPAT_METADATA_CSUM) {
        let s_checksum = &es.s_checksum.to_le_bytes();
        let csum = get_crc32c(s_checksum);
    
        if !verify_crc32c(s_checksum, csum) {
            return Err("Checksum failed".into()); // Make a warning instead of hard failing
        };
    }

    Ok((u32::from_le(es.s_feature_compat), u32::from_le(es.s_feature_incompat), u32::from_le(es.s_feature_ro_compat)))
}

fn ext_get_info(
        ver: u8,
        es: Ext2SuperBlock,
    ) -> Result<(Option<String>, BlockidUUID, Option<BlockidUUID>, Option<FsSecType>, BlockidVersion, u64, u64, u64), Box<dyn std::error::Error>>
{
    let label: Option<String> = if es.s_volume_name[0] != 0 {
        Some(String::from_utf8_lossy(&es.s_volume_name).to_string())
    } else {
        None
    };
    
    let uuid = BlockidUUID::Standard(Uuid::from_bytes(es.s_uuid));
    
    let journal_uuid: Option<BlockidUUID> = if has_compat(es.s_feature_compat, FeatureCompat::EXT3_FEATURE_COMPAT_HAS_JOURNAL) {
        if es.s_journal_uuid == [0; 16] {
            None //Journal is internal to the filesystem   
        } else {
            Some(BlockidUUID::Standard(Uuid::from_bytes(es.s_journal_uuid)))
        }
    } else {
        None
    };

    let sec_type = if ver != 2 && has_incompat(es.s_feature_incompat, FeatureIncompat::EXT2_FEATURE_INCOMPAT_FILETYPE|FeatureIncompat::EXT2_FEATURE_INCOMPAT_FILETYPE) {
        Some(FsSecType::Ext2)
    } else {
        None
    };

    let version = BlockidVersion::DevT(makedev(es.s_rev_level, es.s_minor_rev_level.into()));

    let log_block_size = u32::from_le(es.s_log_block_size);
    assert!(log_block_size < 32, "Shift too large"); 
    let block_size: u64 = (1024u32 << log_block_size).into();
    

    let fslastblock: u64 = u64::from(u32::from_le(es.s_blocks_count))
    | if has_incompat(es.s_feature_incompat, FeatureIncompat::EXT4_FEATURE_INCOMPAT_64BIT) {
        (u64::from(u32::from_le(es.s_blocks_count_hi))) << 32
    } else {
        0
    };

    let fs_size: u64 = block_size * u32::from_le(es.s_blocks_count) as u64; 

    Ok((label, uuid, journal_uuid, sec_type, version, block_size, fslastblock, fs_size))
}

//fn probe_jbd(
//        probe: &mut BlockidProbe, 
//        magic: BlockidMagic
//    ) -> Result<Option<ProbeResult>, Box<dyn std::error::Error>> 
//{
//    Ok(None)
//}

/*
 * reads superblock and returns:
 *	fc = feature_compat
 *	fi = feature_incompat
 *	frc = feature_ro_compat
 */

pub fn probe_ext2(
        probe: &mut BlockidProbe, 
        _magic: BlockidMagic
    ) -> Result<ProbeResult, Box<dyn std::error::Error>> 
{
    let es: Ext2SuperBlock = read_as::<Ext2SuperBlock>(&probe.file, 1024)?;

    let (fc, _, frc) = ext_get_super(es)?;
    
    if has_compat(fc, FeatureCompat::EXT3_FEATURE_COMPAT_HAS_JOURNAL) {
        return Err("Ext is ext3 not ext2".into())
    };

    if has_rocompat(frc, FeatureRoCompat::EXT2_FEATURE_RO_COMPAT_SPARSE_SUPER|
                                        FeatureRoCompat::EXT2_FEATURE_RO_COMPAT_LARGE_FILE|
                                        FeatureRoCompat::EXT2_FEATURE_RO_COMPAT_BTREE_DIR) ||
        has_incompat(frc, FeatureIncompat::EXT2_FEATURE_INCOMPAT_FILETYPE|
                                            FeatureIncompat::EXT2_FEATURE_INCOMPAT_META_BG)
    {
        return Err("Ext contains unsupported feature on ext2".into())                                     
    }

    let (label, uuid, journal_uuid, sec_type, version, block_size, fs_last_block, fs_size) = ext_get_info(2, es)?;

    return Ok(ProbeResult::Filesystem(
                FilesystemResults { fs_type: Some(FsType::Ext2), 
                                    sec_type, 
                                    label, 
                                    fs_uuid: Some(uuid), 
                                    log_uuid: None, 
                                    ext_journal: journal_uuid, 
                                    fs_creator: get_os_creator(es),
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
    ) -> Result<ProbeResult, Box<dyn std::error::Error>> 
{
    let es: Ext2SuperBlock = read_as::<Ext2SuperBlock>(&probe.file, 1024)?;

    let (fc, fi, frc) = ext_get_super(es)?;
    
    if !has_compat(fc, FeatureCompat::EXT3_FEATURE_COMPAT_HAS_JOURNAL) {
        return Err("Ext3 needs journal".into())
    };
    
    if has_rocompat(frc, FeatureRoCompat::EXT2_FEATURE_RO_COMPAT_SPARSE_SUPER|
                                        FeatureRoCompat::EXT2_FEATURE_RO_COMPAT_LARGE_FILE|
                                        FeatureRoCompat::EXT2_FEATURE_RO_COMPAT_BTREE_DIR) ||
        has_incompat(fi, FeatureIncompat::EXT2_FEATURE_INCOMPAT_FILETYPE| 
                                        FeatureIncompat::EXT3_FEATURE_INCOMPAT_RECOVER| 
                                        FeatureIncompat::EXT2_FEATURE_INCOMPAT_META_BG)
    {
        return Err("Ext doesnt contain all supported features of ext3".into())                                     
    }

    let (label, uuid, journal_uuid, sec_type, version, block_size, fs_last_block, fs_size) = ext_get_info(3, es)?;

    return Ok(ProbeResult::Filesystem(
                FilesystemResults { fs_type: Some(FsType::Ext3), 
                                    sec_type, 
                                    label, 
                                    fs_uuid: Some(uuid), 
                                    log_uuid: None, 
                                    ext_journal: journal_uuid, 
                                    fs_creator: get_os_creator(es),
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
    ) -> Result<ProbeResult, Box<dyn std::error::Error>> 
{
    let es: Ext2SuperBlock = read_as::<Ext2SuperBlock>(&probe.file, 1024)?;

    let (_fc, fi, frc) = ext_get_super(es)?;
    
    if has_incompat(fi, FeatureIncompat::EXT3_FEATURE_INCOMPAT_JOURNAL_DEV) {
        return Err("Ext is jbd".into());
    }
        
    if has_rocompat(frc, FeatureRoCompat::EXT2_FEATURE_RO_COMPAT_SPARSE_SUPER|
                                        FeatureRoCompat::EXT2_FEATURE_RO_COMPAT_LARGE_FILE|
                                        FeatureRoCompat::EXT2_FEATURE_RO_COMPAT_BTREE_DIR) ||
        has_incompat(fi, FeatureIncompat::EXT2_FEATURE_INCOMPAT_FILETYPE| 
                                        FeatureIncompat::EXT3_FEATURE_INCOMPAT_RECOVER| 
                                        FeatureIncompat::EXT2_FEATURE_INCOMPAT_META_BG)
    {
        return Err("Ext doesnt contain all supported features of ext4".into())                                     
    }

    if has_ext_flags(u32::from_le(es.s_flags), ExtFlags::EXT2_FLAGS_TEST_FILESYS) {
        return Err("Ext is a ext4 development version".into());
    }

    let (label, uuid, journal_uuid, sec_type, version, block_size, fs_last_block, fs_size) = ext_get_info(4, es)?;

    return Ok(ProbeResult::Filesystem(
                FilesystemResults { fs_type: Some(FsType::Ext4), 
                                    sec_type, 
                                    label, 
                                    fs_uuid: Some(uuid), 
                                    log_uuid: None, 
                                    ext_journal: journal_uuid, 
                                    fs_creator: get_os_creator(es),
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