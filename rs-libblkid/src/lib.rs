use std::u16;
use std::fs::File;
use std::io::{SeekFrom, Seek, Read};

pub enum SuperState {
    CleanlyUnmounted = 0x0001,   // Cleanly unmounted
    ErrorsDetected = 0x0002,      // Errors detected
    OrphansBeingRecovered = 0x0004, // Orphans being recovered
}

pub enum SuperErrors {
    Continue = 1, // Continue
    RemountRO = 2, // Remount read-only
    Panic = 3, // Panic
}

pub enum SuperCreator {
    Linux = 0,
    Hurd = 1,
    Masix = 2, 
    FreeBSD = 3,
    Lites = 4,
}

pub enum SuperRevision {
    OriginalFormat = 1, // Original format
    V2Format = 2, // v2 format w/ dynamic inode sizes
}

pub struct Ext4SuperBlock {
    s_inodes_count: u32,
    s_blocks_count_lo: u32, 
    s_r_blocks_count_lo: u32, 
    s_free_blocks_count_lo: u32,
    s_free_inodes_count: u32, 
    s_first_data_block: u32, 
    s_log_block_size: u32, 
    s_log_cluster_size: u32, 
    s_blocks_per_group: u32, 
    s_clusters_per_group: u32, 
    s_inodes_per_group: u32, 
    s_mtime: u32, 
    s_wtime: u32,
    s_mnt_count: u16, 
    s_max_mnt_count: u16,
    s_magic: u16, 
    s_state: u16, 
    s_errors: u16, 
    s_minor_rev_level: u16, 
    s_lastcheck: u32, 
    s_checkinterval: u32, 
    s_creator_os: u32, 
    s_rev_level: u32, 
    s_def_resuid: u16, 
    s_def_resgid: u16, 
    s_first_ino: u32, 
    s_inode_size: u16, 
    s_block_group_nr: u16, 
    s_feature_compat: u32,
    s_feature_incompat: u32,
    s_feature_ro_compat: u32,
    s_uuid: [u8; 16],
    s_volume_name: [u8; 16], 
    s_last_mounted: [u8; 64],
    s_algorithm_usage_bitmap: u32,
    s_prealloc_blocks: u8,
    s_prealloc_dir_blocks: u8,
    s_reserved_gdt_blocks: u16,
    s_journal_uuid: [u8; 16],
    s_journal_inum: u32, 
    s_journal_dev: u32, 
    s_last_orphan: u32,
    s_hash_seed: [u32; 4],
    s_def_hash_version: u8,
    s_jnl_backup_type: u8,
    s_desc_size: u16,
    s_default_mount_opts: u32,
    s_first_meta_bg: u32,
    s_mkfs_time: u32,
    s_jnl_blocks: [u32; 17],
    s_blocks_count_hi: u32,
    s_r_blocks_count_hi: u32,
    s_free_blocks_count_hi: u32,
    s_min_extra_isize: u16,
    s_want_extra_isize: u16,
    s_flags: u32,
    s_raid_stride: u16,
    s_mmp_interval: u16,
    s_mmp_block: u64,
    s_raid_stripe_width: u32,
    s_log_groups_per_flex: u8,
    s_checksum_type: u8,
    s_encryption_level: u8,
    s_reserved_pad: u8,
    s_kbytes_written: u64,
    s_snapshot_inum: u32,
    s_snapshot_id: u32,
    s_snapshot_r_blocks_count: u64,
    s_snapshot_list: u32,
    s_error_count: u32,
    s_first_error_time: u32,
    s_first_error_ino: u32,
    s_first_error_block: u64,
    s_first_error_func: [u8; 32],
    s_first_error_line: u32,
    s_last_error_time: u32,
    s_last_error_ino: u32,
    s_last_error_line: u32,
    s_last_error_block: u64,
    s_last_error_func: [u8; 32],
    s_mount_opts: [u8; 64],
    s_usr_quota_inum: u32,
    s_grp_quota_inum: u32,
    s_overhead_blocks: u32,
    s_backup_bgs: [u32; 2],
    s_encrypt_algos: [u8; 4],
    s_encrypt_pw_salt: [u8; 16],
    s_lpf_ino: u32,
    s_prj_quota_inum: u32,
    s_checksum_seed: u32,
    s_wtime_hi: u8,
    s_mtime_hi: u8,
    s_mkfs_time_hi: u8,
    s_lastcheck_hi: u8,
    s_first_error_time_hi: u8,
    s_last_error_time_hi: u8,
    s_first_error_errcode: u8,
    s_last_error_errcode: u8,
    s_encoding: u16,
    s_encoding_flags: u16,
    s_orphan_file_inum: u32,
    s_reserved: [u32; 94],
    s_checksum: u32,
}

pub fn read_ext4(device: &str) -> Result<(), Box<dyn std::error::Error>>{
    let mut superblock = File::open(device)?;
    superblock.seek(SeekFrom::Start(1024))?;
    let mut buffer = [0; 1024];
    superblock.read_exact(&mut buffer)?;
    
    for (i, byte) in buffer.iter().enumerate() {
        print!("{:02x} ", byte);
        if (i + 1) % 16 == 0 {
            println!(); // new line every 16 bytes
        }
    }

    let bytes = [0x1d, 0xf5, 0x11, 0x68];
    let num = u32::from_le_bytes(bytes);   
    println!("First 4 bytes: {}", num);  

    return Ok(());
}
