use std::u16;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Cursor};
use byteorder::{LittleEndian, ReadBytesExt};
use uuid::Uuid;

/*
https://www.kernel.org/doc/html/latest/filesystems/ext4/globals.html
*/

#[derive(Debug, Clone, Copy)]
pub enum SuperState {
    CleanlyUnmounted ,  
    ErrorsDetected,      
    OrphansBeingRecovered,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub enum SuperErrors {
    Continue, 
    RemountRO, 
    Panic,
    Unknown, 
}

#[derive(Debug, Clone, Copy)]
pub enum SuperCreator {
    Linux,
    Hurd,
    Masix, 
    FreeBSD,
    Lites,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub enum SuperRevision {
    OriginalFormat,
    V2Format,
    Unknown,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4SuperBlock {
    pub s_inodes_count: u32,
    pub s_blocks_count_lo: u32, 
    pub s_r_blocks_count_lo: u32, 
    pub s_free_blocks_count_lo: u32,
    pub s_free_inodes_count: u32, 
    pub s_first_data_block: u32, 
    pub s_log_block_size: u32, 
    pub s_log_cluster_size: u32, 
    pub s_blocks_per_group: u32, 
    pub s_clusters_per_group: u32, 
    pub s_inodes_per_group: u32, 
    pub s_mtime: u32, 
    pub s_wtime: u32,
    pub s_mnt_count: u16, 
    pub s_max_mnt_count: u16,
    pub s_magic: u16, 
    pub s_state: SuperState, 
    pub s_errors: SuperErrors, 
    pub s_minor_rev_level: SuperRevision, 
    pub s_lastcheck: u32, 
    pub s_checkinterval: u32, 
    pub s_creator_os: SuperCreator, 
    pub s_rev_level: u32, 
    pub s_def_resuid: u16, 
    pub s_def_resgid: u16, 
    pub s_first_ino: u32, 
    pub s_inode_size: u16, 
    pub s_block_group_nr: u16, 
    pub s_feature_compat: u32,
    pub s_feature_incompat: u32,
    pub s_feature_ro_compat: u32,
    pub s_uuid: Uuid,
    pub s_volume_name: [u8; 16], 
    pub s_last_mounted: [u8; 64],
    pub s_algorithm_usage_bitmap: u32,
    pub s_prealloc_blocks: u8,
    pub s_prealloc_dir_blocks: u8,
    pub s_reserved_gdt_blocks: u16,
    pub s_journal_uuid: Uuid,
    pub s_journal_inum: u32, 
    pub s_journal_dev: u32, 
    pub s_last_orphan: u32,
    pub s_hash_seed: [u32; 4],
    pub s_def_hash_version: u8,
    pub s_jnl_backup_type: u8,
    pub s_desc_size: u16,
    pub s_default_mount_opts: u32,
    pub s_first_meta_bg: u32,
    pub s_mkfs_time: u32,
    pub s_jnl_blocks: [u32; 17],
    pub s_blocks_count_hi: u32,
    pub s_r_blocks_count_hi: u32,
    pub s_free_blocks_count_hi: u32,
    pub s_min_extra_isize: u16,
    pub s_want_extra_isize: u16,
    pub s_flags: u32,
    pub s_raid_stride: u16,
    pub s_mmp_interval: u16,
    pub s_mmp_block: u64,
    pub s_raid_stripe_width: u32,
    pub s_log_groups_per_flex: u8,
    pub s_checksum_type: u8,
    pub s_encryption_level: u8,
    pub s_reserved_pad: u8,
    pub s_kbytes_written: u64,
    pub s_snapshot_inum: u32,
    pub s_snapshot_id: u32,
    pub s_snapshot_r_blocks_count: u64,
    pub s_snapshot_list: u32,
    pub s_error_count: u32,
    pub s_first_error_time: u32,
    pub s_first_error_ino: u32,
    pub s_first_error_block: u64,
    pub s_first_error_func: [u8; 32],
    pub s_first_error_line: u32,
    pub s_last_error_time: u32,
    pub s_last_error_ino: u32,
    pub s_last_error_line: u32,
    pub s_last_error_block: u64,
    pub s_last_error_func: [u8; 32],
    pub s_mount_opts: [u8; 64],
    pub s_usr_quota_inum: u32,
    pub s_grp_quota_inum: u32,
    pub s_overhead_blocks: u32,
    pub s_backup_bgs: [u32; 2],
    pub s_encrypt_algos: [u8; 4],
    pub s_encrypt_pw_salt: [u8; 16],
    pub s_lpf_ino: u32,
    pub s_prj_quota_inum: u32,
    pub s_checksum_seed: u32,
    pub s_wtime_hi: u8,
    pub s_mtime_hi: u8,
    pub s_mkfs_time_hi: u8,
    pub s_lastcheck_hi: u8,
    pub s_first_error_time_hi: u8,
    pub s_last_error_time_hi: u8,
    pub s_first_error_errcode: u8,
    pub s_last_error_errcode: u8,
    pub s_encoding: u16,
    pub s_encoding_flags: u16,
    pub s_orphan_file_inum: u32,
    pub s_reserved: [u32; 94],
    pub s_checksum: u32,
}

