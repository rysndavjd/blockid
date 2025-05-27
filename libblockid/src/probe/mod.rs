use uuid::Uuid;
use std::fs::File;
use std::path::Path;
use bitflags::bitflags;
use bytemuck::{from_bytes, Pod};
use std::io::{Read, Seek, SeekFrom};
use rustix::fs::{stat, Stat};

use crate::filesystems::vfat::{VfatExtras, VfatVersion, probe_vfat, VFAT_ID_INFO};
use crate::filesystems::volume_id::{self, VolumeId32, VolumeId64};

bitflags! {
    #[derive(Debug, Clone)]
    pub struct ProbeFlags: u32 {
        const PRIVATE_FD     = 1 << 1; // File descriptor opened by blkid
        const TINY_DEV       = 1 << 2; // <= 1.47MiB, e.g., floppy
        const CDROM_DEV      = 1 << 3; // CD/DVD device
        const NOSCAN_DEV     = 1 << 4; // Do not scan this device
        const MODIF_BUFF     = 1 << 5; // Cached buffer modified
        const OPAL_LOCKED    = 1 << 6; // OPAL self-encrypting drive is locked
        const OPAL_CHECKED   = 1 << 7; // OPAL lock status was checked
    }
}

// Macros are cool mate
macro_rules! set_probe_values_fs {
    ($name:ident, $field:ident, $typ:ty) => {
        pub fn $name(&mut self, value: $typ) {
            self.values.fs.$field = Some(value);
        }
    };
}

macro_rules! set_probe_values {
    ($name:ident, $field:ident, $typ:ty) => {
        pub fn $name(&mut self, value: $typ) {
            self.values.$field = Some(value);
        }
    };
}

macro_rules! is_flags {
    ($name:ident, $flag:expr) => {
        pub fn $name(&mut self) -> bool {
            self.probe_flags.contains($flag)
        }
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Usage {
    Filesystem,
    Raid,
    Crypto,
    Lvm,
    Swap,
    Loop,
    PartTable,
    Part,
    Container,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartTableType {
    Mbr,
    Gpt,
    BsdLabel,
    Other(String)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlkUuid {
    Standard(Uuid),
    VolumeId32(volume_id::VolumeId32),
    VolumeId64(volume_id::VolumeId64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsType {
    Vfat,
    Exfat,  
    Ntfs,  
    Ext2,  
    Ext3,  
    Ext4,
    Xfs,  
    Btrfs,  
    Zfs,  
    F2fs,  
    Hfs,
    HfsPlus,
    Apfs,
    Other(String)
}   

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsSecType {
    Msdos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsVersion {
    Vfat(VfatVersion)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsExtras {
    Vfat(VfatExtras)
}

#[derive(Debug, Clone)]
pub struct FsMetadata {
    pub fs_type: Option<FsType>,
    pub fs_version: Option<FsVersion>,
    pub uuid: Option<BlkUuid>,
    pub uuid_sub: Option<BlkUuid>,
    pub label: Option<String>,
    pub usage: Option<Usage>,
    pub fs_block_size: Option<u64>,
    pub block_size: Option<u64>,
    pub fs_size: Option<u64>,
    pub fs_extras: Option<FsExtras>
}

#[derive(Debug, Clone)]
pub struct ProbeResults {
    pub fs: FsMetadata,                    
    pub sec_type: Option<FsSecType>,
    pub part_uuid: Option<BlkUuid>,
    pub part_name: Option<String>,
    pub part_number: Option<u64>,
    pub part_scheme: Option<PartTableType>,
}

#[derive(Debug)]
pub struct BlockProbe {
    pub file: File,
    pub begin: u64,
    pub end: u64,
    pub devno: Stat,
    pub disk_devno: Stat,
    pub probe_flags: ProbeFlags,
    //pub part_table_values: Option<>
    pub values: ProbeResults,
}

#[derive(Debug, Clone)]
pub struct BlockMagic {
    pub magic: &'static [u8],
    pub len: u64,
    pub b_offset: u64,
}

#[derive(Debug, Clone)]
pub struct BlockId {
    pub name: &'static str,
    pub usage: Option<Usage>,
    //pub flags: Option<>,
    pub minsz: Option<u64>,
    pub magics: &'static [BlockMagic],
}

impl FsMetadata {
    pub fn empty() -> Self {
        FsMetadata { 
            fs_type: None,
            fs_version: None,
            uuid: None,
            uuid_sub: None, 
            label: None, 
            usage: None,
            fs_block_size: None,
            block_size: None,
            fs_size: None,
            fs_extras: None,
        }
    }
}

impl ProbeResults {
    pub fn empty() -> Self {
        ProbeResults {
            fs: FsMetadata::empty(),
            sec_type: None, 
            part_uuid: None, 
            part_name: None, 
            part_number: None, 
            part_scheme: None 
        }
    }
}

impl BlockProbe {
    pub fn new(
            file: File, 
            begin: u64, 
            end: u64, 
            devno: Stat, 
            disk_devno: Stat,
        ) -> Self 
    {
        BlockProbe { 
                file: file,
                begin: begin,
                end: end, 
                devno: devno, 
                disk_devno: disk_devno, 
                probe_flags: ProbeFlags::empty(), 
                values: ProbeResults::empty() 
            }
    }

    is_flags!(is_tiny, ProbeFlags::TINY_DEV);

    set_probe_values_fs!(set_fs_type, fs_type, FsType);
    set_probe_values_fs!(set_fs_version, fs_version, FsVersion);
    set_probe_values_fs!(set_uuid, uuid, BlkUuid);
    set_probe_values_fs!(set_uuid_sub, uuid_sub, BlkUuid);

    pub fn set_label_utf8_lossy(&mut self, label: &[u8]) 
    {
        self.values.fs.label = Some(String::from_utf8_lossy(label).to_string())
    }

    set_probe_values_fs!(set_usage, usage, Usage);
    set_probe_values_fs!(set_fs_extras, fs_extras, FsExtras);
    set_probe_values_fs!(set_fs_block_size, fs_block_size, u64);
    set_probe_values_fs!(set_block_size, block_size, u64);
    set_probe_values_fs!(set_fs_size, fs_size, u64);
    set_probe_values!(set_sec_type, sec_type, FsSecType);

}

pub type FsProbeFn = fn(&mut BlockProbe, BlockMagic) -> Result<(), Box<dyn std::error::Error>>;

pub struct FsProbeEntry {
    pub id: &'static BlockId,
    pub probe_fn: FsProbeFn,
}

pub static FS_PROBE_CHAIN: &[FsProbeEntry] = &[
    FsProbeEntry {
        id: &VFAT_ID_INFO,
        probe_fn: probe_vfat,
    },
];

pub fn get_buffer(
        probe: &mut BlockProbe,
        offset: u64,
        buffer_size: usize,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> 
{
    let mut block = probe.file.try_clone()?;
    block.seek(SeekFrom::Start(0))?;

    let mut buffer = vec![0u8; buffer_size];
    block.seek(SeekFrom::Start(offset))?;
    block.read_exact(&mut buffer)?;

    return Ok(buffer);
}

pub fn get_sector(
        probe: &mut BlockProbe,
        sector: u64,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> 
{
    get_buffer(probe, sector << 9, 0x200)
}

pub fn probe_get_magic(
        probe: &mut BlockProbe, 
        id_info: &BlockId
    ) -> Result<BlockMagic, Box<dyn std::error::Error>>
{
    for magic in id_info.magics {
        let b_offset: u64 = magic.b_offset;
        let magic_len: usize = magic.len.try_into().unwrap(); // FIX

        let mut raw = probe.file.try_clone()?;
        raw.seek(SeekFrom::Start(b_offset))?;

        let mut buffer = vec![0; magic_len];

        raw.read_exact(&mut buffer)?;

        if buffer == magic.magic {
            return Ok(magic.clone());
        }
    }
    return Err("Unable to find any magic".into());
}

pub fn read_as<T: Pod>(
        raw_block: &File,
        offset: u64,
    ) -> Result<T, Box<dyn std::error::Error>> 
{
    let mut block = raw_block.try_clone()?;
    block.seek(SeekFrom::Start(0))?;

    let mut buffer = vec![0u8; std::mem::size_of::<T>()];
    block.seek(SeekFrom::Start(offset))?;
    block.read_exact(&mut buffer)?;

    let ptr = from_bytes::<T>(&buffer);
    Ok(*ptr)
}

pub fn get_dev_t<P: AsRef<Path>>(path: P) -> Option<u64> {
    let stat: Stat = stat(path.as_ref()).ok()?;
    Some(stat.st_rdev) 
}

pub fn get_disk_devno<P: AsRef<Path>>(path: P) -> Option<u64> {
    let stat: Stat = stat(path.as_ref()).ok()?;
    Some(stat.st_dev) 
}

fn probe_from_filename(filename: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(filename)?;
    
    //let probe = BlockProbe::new(file, 0, 0, Stat::from(2), disk_devno)

    return Ok(());
}