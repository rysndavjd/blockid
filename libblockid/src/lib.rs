pub mod partitions;
pub mod filesystems;
pub mod probe;

use std::fs::File;
use bitflags::bitflags;
use filesystems::volume_id::{VolumeId32, VolumeId64};
use rustix::fs::{Dev, Mode};
use uuid::Uuid;

#[derive(Debug)]
pub struct BlockidProbe {
    pub file: File,
    pub begin: u64,
    pub end: u64,
    pub io_size: u64,

    pub devno: Dev,
    pub disk_devno: Dev,
    pub sector_size: u64,
    pub mode: Mode,

    pub flags: BlockidFlags,
    pub values: ProbeResult
}

bitflags! {
    #[derive(Debug)]
    struct BlockidFlags: u32 {
        const PRIVATE_FD = 1 << 1;
        const TINY_DEV = 1 << 2;
        const CDROM_DEV = 1 << 3;
        const NOSCAN_DEV = 1 << 4;
        const MODIF_BUFF = 1 << 5;
        const OPAL_LOCKED = 1 << 6;
        const OPAL_CHECKED = 1 << 7;
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
    pub fs_creator: Option<String>,
    pub usage: Option<String>,
    pub version: Option<String>,
    pub sbmagic: Option<String>,
    pub sbmagic_offset: Option<String>,
    pub fs_size: Option<u64>,
    pub fs_last_block: Option<u64>,
    pub fs_block_size: Option<u64>,
    pub block_size: Option<u64>,
}

impl ProbeResult {
    pub fn pt_type(&mut self, pt_type: Option<String>) {
        self.pt_type = pt_type
    }
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

struct BlockidIdinfo {
    name: Option<&'static str>,
    usage: Option<Usage>,
    minsz: Option<u64>,
    probe_fn: ProbeFn,
    magics: &'static [BlockidMagic],
}

bitflags! {
    struct Usage: u32 {
        const FILESYSTEM    = 1 << 1;
        const RAID          = 1 << 2;
        const CRYPTO        = 1 << 3;
        const OTHER         = 1 << 4;
    }
}

pub type ProbeFn = fn(&mut BlockidProbe, BlockidMagic) -> Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, Clone, Copy)]
pub struct BlockidMagic {
    pub magic: &'static [u8],
    pub len: u64,
    pub b_offset: u64,
}
