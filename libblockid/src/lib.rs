// Partition Tables
pub mod mbr;
pub mod gpt;

// Filesystems
pub mod ext4;
pub mod vfat;

use uuid::Uuid;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::prelude::{AsRawFd, RawFd};
use std::result;
use nix::sys::stat::dev_t;
use bitflags::bitflags;

/*
ideas

make a probe function with a struct
that has basic info for filesystems and partitions eg like
struct filesystems
    file system uuid
    partition uuid 
    label
    filesystem type
    filesystem version 
    filesystem magic signature
    size of filesystem in bytes
    
struct partitions
    partition type
    partition table uuid/id
    partition name
    partition uuid
    partition number
    partition offset
    partition size
    disk maj:min
*/

bitflags! {
    #[derive(Debug, Clone)]
    pub struct BlkidFlags: u32 {
        const PRIVATE_FD     = 1 << 1; // File descriptor opened by blkid
        const TINY_DEV       = 1 << 2; // <= 1.47MiB, e.g., floppy
        const CDROM_DEV      = 1 << 3; // CD/DVD device
        const NOSCAN_DEV     = 1 << 4; // Do not scan this device
        const MODIF_BUFF     = 1 << 5; // Cached buffer modified
        const OPAL_LOCKED    = 1 << 6; // OPAL self-encrypting drive is locked
        const OPAL_CHECKED   = 1 << 7; // OPAL lock status was checked
    }
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
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartTableType {
    Mbr,
    Gpt,
    BsdLabel,
    Unknown
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsUuid {
    Standard(Uuid),
    VolumeId32(u32),
    VolumeId64(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsType {
    Fat32,
    Fat16,
    Fat12,
    Fat,
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
}   

#[derive(Debug, Clone)]
pub struct Partition {
}

#[derive(Debug, Clone)]
pub struct FilesystemResults {
    filesystem: Option<FsType>,
    uuid: Option<FsUuid>,
    uuid_sub: Option<FsUuid>,
    label: Option<String>,
    fs_version: Option<String>,
    usage: Option<Usage>,
}

#[derive(Debug, Clone)]
pub struct ProbeResults {
    fs_type: Option<FsType>,
    sec_type: Option<FsType>,
    uuid: Option<FsUuid>,
    uuid_sub: Option<FsUuid>,
    label: Option<String>,
    label_raw: Option<Vec<u8>>,
    fs_version: Option<String>,
    usage: Option<Usage>,
    part_uuid: Option<FsUuid>,
    part_name: Option<String>,
    part_number: Option<u64>,
    part_scheme: Option<PartTableType>
}

#[derive(Debug)]
pub struct BlockProbe {
    pub file: File,
    pub begin: u64,
    pub end: u64,
    pub devno: dev_t,
    pub disk_devno: dev_t,
    pub probe_flags: BlkidFlags,
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
    pub usage: Usage,
    pub magics: &'static [BlockMagic],
}

impl BlockProbe {
    fn is_tiny(&self) -> bool {
        self.probe_flags.contains(BlkidFlags::TINY_DEV)
    }

    fn set_label_from_string(&mut self, label: String) {
        self.values.label = Some(label)
    }

    fn set_label_from_bytes(&mut self, label: Vec<u8>) {
        self.values.label = Some(String::from_utf8_lossy(&label).to_string())
    }

    fn set_uuid(&mut self, uuid: Uuid) {
        self.values.uuid = Some(FsUuid::Standard(uuid))
    }

    fn set_32bit_volume_id(&mut self, uuid: u32) {
        self.values.uuid = Some(FsUuid::VolumeId32(uuid))
    }

    fn set_64bit_volume_id(&mut self, uuid: u64) {
        self.values.uuid = Some(FsUuid::VolumeId64(uuid))
    }
}

fn is_power_2(num: u64) -> bool {
    return num != 0 && ((num & (num - 1)) == 0); 
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

fn probe_type() {
    todo!()
}

/* 
pub fn probe_from_filename(filename: &str) -> Result<BlockProbe, Box<dyn std::error::Error>> {
    let block_file = File::open(filename)?;
    let fd = block_file.as_raw_fd();



    return Ok(BlockProbe { 
        fd: fd, 
        begin: (), 
        end: (), 
        devno: (), 
        disk_devno: (), 
        values: () 
    });
}
*/

pub fn read_raw(device: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut superblock = File::open(device)?;
    //superblock.seek(SeekFrom::Start(65536))?;
    let mut buffer = [0; 512];
    superblock.read_exact(&mut buffer)?;

    println!("{:X?}", buffer);
    return Ok(());

}
