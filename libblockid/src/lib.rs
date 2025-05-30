pub mod partitions;
pub mod filesystems;
pub mod probe;

use std::{fs::File, slice::SliceIndex};
use bitflags::bitflags;
use filesystems::volume_id::{VolumeId32, VolumeId64};
use rustix::fs::{Dev, Mode};
use uuid::Uuid;

#[derive(Debug)]
pub struct BlockidProbe {
    pub file: File,
    //pub begin: u64,
    //pub end: u64,
    //pub io_size: u64,

    //pub devno: Dev,
    //pub disk_devno: Dev,
    //pub sector_size: u64,
    //pub mode: Mode,

    pub flags: BlockidFlags,
    pub pt_values: Option<ProbePtResult>,
    pub fs_values: Option<ProbeFsResult>,
}

impl BlockidProbe {
    pub fn new(file: File) -> Self {
        BlockidProbe { file: file, flags: BlockidFlags::empty(), pt_values: Some(ProbePtResult::empty()), fs_values: Some(ProbeFsResult::empty())}
    }
}

bitflags! {
    #[derive(Debug)]
    pub struct BlockidFlags: u32 {
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
pub struct ProbePtResult {
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
}

impl ProbePtResult {
    pub fn empty() -> Self {
        ProbePtResult { pt_type: None, pt_uuid: None, part_entry_scheme: None, part_entry_name: None, part_entry_uuid: None, part_entry_number: None, part_entry_offset: None, part_entry_size: None, part_entry_disk: None }
    }
}

#[derive(Debug)]
pub struct ProbeFsResult {
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


impl ProbeFsResult {
    pub fn empty() -> Self {
        ProbeFsResult { fs_type: None, sec_type: None, label: None, fs_uuid: None, log_uuid: None, ext_journal: None, fs_creator: None, usage: None, version: None, sbmagic: None, sbmagic_offset: None, fs_size: None, fs_last_block: None, fs_block_size: None, block_size: None }
    }
}

#[derive(Debug)]
pub enum BlockidPtUUID {
    Standard(Uuid),
    VolumeId32(VolumeId32),
    VolumeId64(VolumeId64)
}

#[derive(Debug)]
pub enum BlockidFsUUID {
    Standard(Uuid),
    VolumeId32(VolumeId32),
    VolumeId64(VolumeId64)
}

pub struct BlockidIdinfo {
    pub name: Option<&'static str>,
    pub usage: Option<Usage>,
    pub minsz: Option<u64>,
    pub probe_fn: ProbeFn,
    pub magics: &'static [BlockidMagic],
}

pub enum Usage {
    Filesystem,
    PartitionTable,
    Raid,
    Crypto,
    Other(String),
}

pub type ProbeFn = fn(&mut BlockidProbe, BlockidMagic) -> Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, Clone, Copy)]
pub struct BlockidMagic {
    pub magic: &'static [u8],
    pub len: u64,
    pub b_offset: u64,
}
