use std::u16;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Cursor};
use byteorder::ByteOrder;
use rustix::fs::makedev;
use uuid::Uuid;
use bytemuck::{Pod, Zeroable};
use bitflags::bitflags;

use crate::crc32c::{verify_crc32c, get_crc32c};
use crate::{probe_get_magic, read_as, read_buffer_vec, FsSecType};
use crate::{BlockidMagic, BlockidIdinfo, UsageType, BlockidProbe, ProbeResult, BlockidUUID, BlockidVersion};

/*
https://www.kernel.org/doc/html/latest/filesystems/ext4/globals.html
*/

pub const JBD_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("jbd"),
    usage: Some(UsageType::Other("jbd")),
    probe_fn: probe_jbd,
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
    probe_fn: probe_ext2,
    minsz: None,
    magics: &[
        BlockidMagic {
            magic: &[0x53, 0xEF],
            len: 2,
            b_offset: 0x38,
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
            magic: &[0x53, 0xEF],
            len: 2,
            b_offset: 0x38,
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
            magic: &[0x53, 0xEF],
            len: 2,
            b_offset: 0x38,
        },
    ]
};

pub const EXT4DEV_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("ext4dev"),
    usage: Some(UsageType::Filesystem),
    probe_fn: probe_ext4dev,
    minsz: None,
    magics: &[
        BlockidMagic {
            magic: &[0x53, 0xEF],
            len: 2,
            b_offset: 0x38,
        },
    ]
};

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

/*
 * reads superblock and returns:
 *	fc = feature_compat
 *	fi = feature_incompat
 *	frc = feature_ro_compat
 */
fn ext_get_super(
        probe: &mut BlockidProbe
    ) -> Result<(u32, u32, u32), Box<dyn std::error::Error>>
{   
    let es = read_as::<Ext2SuperBlock>(&probe.file, 0)?;
    
    if FeatureRoCompat::from_bits_truncate(es.s_feature_ro_compat).contains(FeatureRoCompat::EXT4_FEATURE_RO_COMPAT_METADATA_CSUM) {
        let csum = get_crc32c(&es.s_checksum.to_le_bytes());
    
        if !verify_crc32c(&es.s_checksum.to_le_bytes(), csum) {
            return Err("Checksum failed".into());
        };
    }

    Ok((es.s_feature_compat.to_le(), es.s_feature_incompat.to_le() ,es.s_feature_ro_compat.to_le()))
}

fn ext_get_info(
        probe: &mut BlockidProbe,
        ver: u8,
        es: Ext2SuperBlock,
    ) -> Result<(Option<String>, BlockidUUID, Option<BlockidUUID>, Option<FsSecType>, BlockidVersion, u64), Box<dyn std::error::Error>>
{
    let s_feature_incompat = FeatureIncompat::from_bits_truncate(es.s_feature_incompat);

    let label: Option<String> = if es.s_volume_name[0] != 0 {
        Some("".to_string())
    } else {
        None
    };
    
    let uuid = Uuid::from_bytes(es.s_uuid);

    let journal_uuid: Option<BlockidUUID> = if FeatureCompat::from_bits_truncate(es.s_feature_compat).contains(FeatureCompat::EXT3_FEATURE_COMPAT_HAS_JOURNAL) {
        Some(BlockidUUID::Standard(Uuid::from_bytes(es.s_journal_uuid)))
    } else {
        None
    };

    let sec_type = if ver != 2 && s_feature_incompat.contains(FeatureIncompat::EXT2_FEATURE_INCOMPAT_FILETYPE|FeatureIncompat::EXT2_FEATURE_INCOMPAT_FILETYPE) {
        Some(FsSecType::Ext2)
    } else {
        None
    };

    let version = makedev(es.s_rev_level, es.s_minor_rev_level.into());

    let log_block_size = u32::from_le(es.s_log_block_size);
    assert!(log_block_size <= 31, "Shift too large"); 
    let block_size = 1024u32 << log_block_size;

    let fs_size: u64 = block_size as u64 * u32::from_le(es.s_blocks_count) as u64; 

    Ok((label, BlockidUUID::Standard(uuid), journal_uuid, sec_type, BlockidVersion::DevId(version), fs_size))
}

fn probe_jbd(
        probe: &mut BlockidProbe, 
        magic: BlockidMagic
    ) -> Result<Option<ProbeResult>, Box<dyn std::error::Error>> 
{
    Ok(None)
}

fn probe_ext2(
        probe: &mut BlockidProbe, 
        magic: BlockidMagic
    ) -> Result<Option<ProbeResult>, Box<dyn std::error::Error>> 
{
    Ok(None)
}

fn probe_ext3(
        probe: &mut BlockidProbe, 
        magic: BlockidMagic
    ) -> Result<Option<ProbeResult>, Box<dyn std::error::Error>> 
{
    Ok(None)
}

fn probe_ext4(
        probe: &mut BlockidProbe, 
        magic: BlockidMagic
    ) -> Result<Option<ProbeResult>, Box<dyn std::error::Error>> 
{
    Ok(None)
}

fn probe_ext4dev(
        probe: &mut BlockidProbe, 
        magic: BlockidMagic
    ) -> Result<Option<ProbeResult>, Box<dyn std::error::Error>> 
{
    Ok(None)
}