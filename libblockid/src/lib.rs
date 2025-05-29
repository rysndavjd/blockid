pub mod partitions;
pub mod filesystems;
pub mod probe;

use std::fs::File;
use bitflags::bitflags;
use filesystems::volume_id::{VolumeId32, VolumeId64};
use rustix::fs::{Dev, Mode};
use uuid::Uuid;

#[derive(Debug)]
struct BlockidProbe {
    file: File,
    begin: u64,
    end: u64,
    io_size: u64,

    devno: Dev,
    disk_devno: Dev,
    sector_size: u64,
    mode: Mode,
    zone_size: u64,

    flags: BlockidFlags,
    prob_flags: BlockidProbFlags,
    
    values: ProbeResult
}

bitflags! {
    #[derive(Debug)]
    struct BlockidFlags: u32 {
        const BLKID_FL_PRIVATE_FD = 1 << 1;
        const BLKID_FL_TINY_DEV = 1 << 2;
        const BLKID_FL_CDROM_DEV = 1 << 3;
        const BLKID_FL_NOSCAN_DEV = 1 << 4;
        const BLKID_FL_MODIF_BUFF = 1 << 5;
        const BLKID_FL_OPAL_LOCKED = 1 << 6;
        const BLKID_FL_OPAL_CHECKED = 1 << 7;
    }
}

bitflags! {
    #[derive(Debug)]
    struct BlockidProbFlags: u32 {
        const BLKID_PROBE_FL_IGNORE_PT = 1 << 1;
    }
}

#[derive(Debug)]
pub struct ProbeResult {
    pub pt_type: Option<String>,
    pub pt_uuid: Option<BlockidPtUUID>,
    pub part_entry_scheme: Option<String>,
    pub part_entry_name: Option<String>,
    pub part_entry_uuid: Option<String>,
    //pub part_entry_type: Option<BlockidPartEntryType>,
    //pub part_entry_flags: Option<String>,
    pub part_entry_number: Option<u64>,
    pub part_entry_offset: Option<u64>,
    pub part_entry_size: Option<u64>,
    pub part_entry_disk: Option<Dev>,

    pub fs_type: Option<String>,
    pub sec_type: Option<String>,
    pub label: Option<String>,
    //pub label_raw: Option<String>,
    pub fs_uuid: Option<BlockidFsUUID>,
    //pub fs_uuid_raw: Option<String>,
    pub log_uuid: Option<String>,
    //pub log_uuid_raw: Option<String>,
    pub ext_journal: Option<String>,
    pub usage: Option<String>,
    pub version: Option<String>,
    pub sbmagic: Option<String>,
    pub sbmagic_offset: Option<String>,
    pub fs_size: Option<u64>,
    pub fs_last_block: Option<u64>,
    pub fs_block_size: Option<u64>,
    pub block_size: Option<u64>,
}

#[derive(Debug)]
enum BlockidPtUUID {
    Standard(Uuid),
    VolumeId32(VolumeId32),
    VolumeId64(VolumeId64)
}

#[derive(Debug)]
enum BlockidFsUUID {
    Standard(Uuid),
    VolumeId32(VolumeId32),
    VolumeId64(VolumeId64)
}


//#[derive(Debug)]
//enum BlockidPartEntryType {
//    Hex(u8),
//    UuidType(Uuid),
//    Other(Vec<u8>), 
//}

struct BlockidIdinfo {
    name: Option<&'static str>,
    usage: Option<UsageFlags>,
    flags: Option<IdInfoFlags>,
    minsz: Option<u64>,
    probe_fn: FsProbeFn,
    magics: &'static [BlockMagic],
}

bitflags! {
    struct UsageFlags: u32 {
        const FILESYSTEM    = 1 << 1;
        const RAID          = 1 << 2;
        const CRYPTO        = 1 << 3;
        const OTHER         = 1 << 4;
    }
}

bitflags! {
    struct IdInfoFlags: u32 {
        const BLKID_IDINFO_TOLERANT    = 1 << 1;
    }
}

pub type FsProbeFn = fn(&mut BlockidProbe, BlockMagic) -> Result<(), Box<dyn std::error::Error>>;

#[derive(Debug)]
pub struct BlockMagic {
    pub magic: &'static [u8],
    pub len: u64,
    pub b_offset: u64,
}

pub enum ChainId {
    Sublks,   
    Toplogy,  
    Parts,    
}

pub trait ChainDriver {
    fn id(&self) -> ChainId;
    fn name(&self) -> &'static str;
    fn default_flags(&self) -> u32;
    fn default_enabled(&self) -> bool;
    fn has_filter(&self) -> bool;

    fn probe(&self, probe: &mut BlockidProbe) -> Result<(), Box<dyn std::error::Error>>;
    fn safe_probe(&self, probe: &mut BlockidProbe) -> Result<(), Box<dyn std::error::Error>>;
}
