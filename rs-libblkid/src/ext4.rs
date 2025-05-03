use std::u16;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Cursor};
use byteorder::{LittleEndian, ReadBytesExt};
use uuid::Uuid;

/*
Used this linux kernel documentation to figure out the structure of ext4
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

/*  
Checks if given device is actual ext4 by reading its magic sig 0xEF53
*/
pub fn check_ext4(device: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut superblock = File::open(device)?;
    // Move cursor forward 1024 bytes not to break GPT/MBR stuff
    // then extra 56 bytes to get at the offset where ext4 magic sig is
    superblock.seek(SeekFrom::Start(1024+56))?;
    let mut buffer = [0; 2];
    superblock.read_exact(&mut buffer)?;

    let mut rdr = Cursor::new(buffer);
    if rdr.read_u16::<LittleEndian>()? == 0xEF53 {
        return Ok(());
    } else {
        return Err("Device given is not a ext4 filesystem".into());
    };
}

/*
Reads ext4 header/structure and returns its contents in a Struct
*/
pub fn read_ext4(device: &str) -> Result<Ext4SuperBlock, Box<dyn std::error::Error>>{
    check_ext4(device)?;
    
    let mut superblock = File::open(device)?;
    // Move cursor forward 1024 bytes not to break GPT/MBR stuff
    superblock.seek(SeekFrom::Start(1024))?;
    let mut buffer = [0; 1024];
    superblock.read_exact(&mut buffer)?;
 
    let mut rdr = Cursor::new(buffer);

    return Ok(
        Ext4SuperBlock {
            s_inodes_count: rdr.read_u32::<LittleEndian>()?,
            s_blocks_count_lo: rdr.read_u32::<LittleEndian>()?, 
            s_r_blocks_count_lo: rdr.read_u32::<LittleEndian>()?, 
            s_free_blocks_count_lo: rdr.read_u32::<LittleEndian>()?,
            s_free_inodes_count: rdr.read_u32::<LittleEndian>()?, 
            s_first_data_block: rdr.read_u32::<LittleEndian>()?, 
            s_log_block_size: rdr.read_u32::<LittleEndian>()?, 
            s_log_cluster_size: rdr.read_u32::<LittleEndian>()?, 
            s_blocks_per_group: rdr.read_u32::<LittleEndian>()?, 
            s_clusters_per_group: rdr.read_u32::<LittleEndian>()?, 
            s_inodes_per_group: rdr.read_u32::<LittleEndian>()?, 
            s_mtime: rdr.read_u32::<LittleEndian>()?, 
            s_wtime: rdr.read_u32::<LittleEndian>()?,
            s_mnt_count: rdr.read_u16::<LittleEndian>()?, 
            s_max_mnt_count: rdr.read_u16::<LittleEndian>()?,
            s_magic: rdr.read_u16::<LittleEndian>()?, 
            s_state: {  
                let state = rdr.read_u16::<LittleEndian>()?;
                match state {
                    0x0001 => SuperState::CleanlyUnmounted,
                    0x0002 => SuperState::ErrorsDetected,
                    0x0004 => SuperState::OrphansBeingRecovered,
                    _ => SuperState::Unknown,
                }
            }, 
            s_errors: {
                let errors = rdr.read_u16::<LittleEndian>()?;
                match errors {
                    1 => SuperErrors::Continue,
                    2 => SuperErrors::RemountRO,
                    3 => SuperErrors::Panic,
                    _ => SuperErrors::Unknown,
                }
            }, 
            s_minor_rev_level: {
                let revision = rdr.read_u16::<LittleEndian>()?;
                match revision {
                    1 => SuperRevision::OriginalFormat,
                    2 => SuperRevision::V2Format,
                    _ => SuperRevision::Unknown,
                } 
            },
            s_lastcheck: rdr.read_u32::<LittleEndian>()?, 
            s_checkinterval: rdr.read_u32::<LittleEndian>()?, 
            s_creator_os: {
                let creator = rdr.read_u32::<LittleEndian>()?;
                match creator {
                    0 => SuperCreator::Linux,
                    1 => SuperCreator::Hurd,
                    2 => SuperCreator::Masix,
                    3 => SuperCreator::FreeBSD,
                    4 => SuperCreator::Lites,
                    _ => SuperCreator::Unknown,
                }
            }, 
            s_rev_level: rdr.read_u32::<LittleEndian>()?, 
            s_def_resuid: rdr.read_u16::<LittleEndian>()?, 
            s_def_resgid: rdr.read_u16::<LittleEndian>()?, 
            s_first_ino: rdr.read_u32::<LittleEndian>()?, 
            s_inode_size: rdr.read_u16::<LittleEndian>()?, 
            s_block_group_nr: rdr.read_u16::<LittleEndian>()?, 
            s_feature_compat: rdr.read_u32::<LittleEndian>()?,
            s_feature_incompat: rdr.read_u32::<LittleEndian>()?,
            s_feature_ro_compat: rdr.read_u32::<LittleEndian>()?,
            s_uuid: {
                let mut uuid = [0u8; 16];
                rdr.read_exact(&mut uuid)?;
                let hex_string: String = uuid
                    .iter()
                    .map(|&b| format!("{:02x}", b))
                    .collect();
                Uuid::try_parse(&hex_string)?
            },
            s_volume_name: {
                let mut volume_name = [0u8; 16];
                rdr.read_exact(&mut volume_name)?;
                volume_name
            },
            s_last_mounted: {
                let mut last_mounted = [0u8; 64];
                rdr.read_exact(&mut last_mounted)?;
                last_mounted
            },
            s_algorithm_usage_bitmap: rdr.read_u32::<LittleEndian>()?,
            s_prealloc_blocks: rdr.read_u8()?,
            s_prealloc_dir_blocks: rdr.read_u8()?,
            s_reserved_gdt_blocks: rdr.read_u16::<LittleEndian>()?,
            s_journal_uuid: {
                let mut journal_uuid = [0u8; 16];
                rdr.read_exact(&mut journal_uuid)?;
                let hex_string: String = journal_uuid
                    .iter()
                    .map(|&b| format!("{:02x}", b))
                    .collect();
                Uuid::try_parse(&hex_string)?
            },            
            s_journal_inum: rdr.read_u32::<LittleEndian>()?, 
            s_journal_dev: rdr.read_u32::<LittleEndian>()?, 
            s_last_orphan: rdr.read_u32::<LittleEndian>()?,
            s_hash_seed: {
                let mut hash_seed = [0u32; 4];
                for i in 0..4 {
                    hash_seed[i] = rdr.read_u32::<LittleEndian>()?;
                }
                hash_seed
            },            s_def_hash_version: rdr.read_u8()?,
            s_jnl_backup_type: rdr.read_u8()?,
            s_desc_size: rdr.read_u16::<LittleEndian>()?,
            s_default_mount_opts: rdr.read_u32::<LittleEndian>()?,
            s_first_meta_bg: rdr.read_u32::<LittleEndian>()?,
            s_mkfs_time: rdr.read_u32::<LittleEndian>()?,
            s_jnl_blocks: {
                let mut jnl_blocks = [0u32; 17];
                for i in 0..17 {
                    jnl_blocks[i] = rdr.read_u32::<LittleEndian>()?;
                }
                jnl_blocks
            },     
            s_blocks_count_hi: rdr.read_u32::<LittleEndian>()?,
            s_r_blocks_count_hi: rdr.read_u32::<LittleEndian>()?,
            s_free_blocks_count_hi: rdr.read_u32::<LittleEndian>()?,
            s_min_extra_isize: rdr.read_u16::<LittleEndian>()?,
            s_want_extra_isize: rdr.read_u16::<LittleEndian>()?,
            s_flags: rdr.read_u32::<LittleEndian>()?,
            s_raid_stride: rdr.read_u16::<LittleEndian>()?,
            s_mmp_interval: rdr.read_u16::<LittleEndian>()?,
            s_mmp_block: rdr.read_u64::<LittleEndian>()?,
            s_raid_stripe_width: rdr.read_u32::<LittleEndian>()?,
            s_log_groups_per_flex: rdr.read_u8()?,
            s_checksum_type: rdr.read_u8()?,
            s_encryption_level: rdr.read_u8()?,
            s_reserved_pad: rdr.read_u8()?,
            s_kbytes_written: rdr.read_u64::<LittleEndian>()?,
            s_snapshot_inum: rdr.read_u32::<LittleEndian>()?,
            s_snapshot_id: rdr.read_u32::<LittleEndian>()?,
            s_snapshot_r_blocks_count: rdr.read_u64::<LittleEndian>()?,
            s_snapshot_list: rdr.read_u32::<LittleEndian>()?,
            s_error_count: rdr.read_u32::<LittleEndian>()?,
            s_first_error_time: rdr.read_u32::<LittleEndian>()?,
            s_first_error_ino: rdr.read_u32::<LittleEndian>()?,
            s_first_error_block: rdr.read_u64::<LittleEndian>()?,
            s_first_error_func: {
                let mut first_error_func = [0u8; 32];
                for i in 0..32 {
                    first_error_func[i] = rdr.read_u8()?;
                }
                first_error_func
            },   
            s_first_error_line: rdr.read_u32::<LittleEndian>()?,
            s_last_error_time: rdr.read_u32::<LittleEndian>()?,
            s_last_error_ino: rdr.read_u32::<LittleEndian>()?,
            s_last_error_line: rdr.read_u32::<LittleEndian>()?,
            s_last_error_block: rdr.read_u64::<LittleEndian>()?,
            s_last_error_func: {
                let mut last_error_func = [0u8; 32];
                for i in 0..32 {
                    last_error_func[i] = rdr.read_u8()?;
                }
                last_error_func
            },
            s_mount_opts: {
                let mut mount_opts = [0u8; 64];
                for i in 0..64 {
                    mount_opts[i] = rdr.read_u8()?;
                }
                mount_opts
            },
            s_usr_quota_inum: rdr.read_u32::<LittleEndian>()?,
            s_grp_quota_inum: rdr.read_u32::<LittleEndian>()?,
            s_overhead_blocks: rdr.read_u32::<LittleEndian>()?,
            s_backup_bgs: {
                let mut s_backup_bgs = [0u32; 2];
                for i in 0..2 {
                    s_backup_bgs[i] = rdr.read_u32::<LittleEndian>()?;
                }
                s_backup_bgs
            },
            s_encrypt_algos: {
                let mut encrypt_algos = [0u8; 4];
                for i in 0..4 {
                    encrypt_algos[i] = rdr.read_u8()?;
                }
                encrypt_algos
            },
            s_encrypt_pw_salt: {
                let mut encrypt_pw_salt = [0u8; 16];
                for i in 0..16 {
                    encrypt_pw_salt[i] = rdr.read_u8()?;
                }
                encrypt_pw_salt
            },
            s_lpf_ino: rdr.read_u32::<LittleEndian>()?,
            s_prj_quota_inum: rdr.read_u32::<LittleEndian>()?,
            s_checksum_seed: rdr.read_u32::<LittleEndian>()?,
            s_wtime_hi: rdr.read_u8()?,
            s_mtime_hi: rdr.read_u8()?,
            s_mkfs_time_hi: rdr.read_u8()?,
            s_lastcheck_hi: rdr.read_u8()?,
            s_first_error_time_hi: rdr.read_u8()?,
            s_last_error_time_hi: rdr.read_u8()?,
            s_first_error_errcode: rdr.read_u8()?,
            s_last_error_errcode: rdr.read_u8()?,
            s_encoding: rdr.read_u16::<LittleEndian>()?,
            s_encoding_flags: rdr.read_u16::<LittleEndian>()?,
            s_orphan_file_inum: rdr.read_u32::<LittleEndian>()?,
            s_reserved: {
                let mut reserved = [0u32; 94];
                for i in 0..94 {
                    reserved[i] = rdr.read_u32::<LittleEndian>()?;
                }
                reserved
            },
            s_checksum: rdr.read_u32::<LittleEndian>()?,
        }
    );
}
