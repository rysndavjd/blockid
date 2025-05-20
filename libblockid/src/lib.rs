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
/* 
    need to think of better way of storing magic sigs
impl FsType {
    pub fn magic(&self) -> Option<Vec<u8>> {
        match self {
            FsType::Fat32 => None,
            FsType::Fat16 => None,
            FsType::Fat12 => None,
            FsType::Fat => None,
            FsType::Exfat => None,
            FsType::Ntfs => None,
            FsType::Ext2 => Some(vec![0x53, 0xEF]),
            FsType::Ext3 => Some(vec![0x53, 0xEF]), 
            FsType::Ext4 => Some(vec![0x53, 0xEF]), 
            FsType::Xfs => Some(vec![0]),
            FsType::Btrfs => Some(vec![0x5F, 0x42, 0x48, 0x52, 0x66, 0x53, 0x5F, 0x4D]),
            FsType::Zfs => Some(vec![0]),
            FsType::F2fs => Some(vec![0]),
            FsType::Hfs => Some(vec![0]),
            FsType::HfsPlus => Some(vec![0]),
            FsType::Apfs => Some(vec![0]),
        }
    }
}

*/

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
    filesystem: Option<FsType>,
    uuid: Option<FsUuid>,
    uuid_sub: Option<FsUuid>,
    label: Option<String>,
    fs_version: Option<String>,
    usage: Option<Usage>,
    part_uuid: Option<FsUuid>,
    part_name: Option<String>,
    part_number: Option<u64>,
    part_scheme: Option<PartTableType>
}

#[derive(Debug, Clone)]
struct BlockProbe {
    fd: RawFd,
    begin: u64,
    end: u64,
    devno: dev_t,
    disk_devno: dev_t,
    //probe_flags: ProbeFlags,
    values: ProbeResults,
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


pub fn probe_get_magic(raw: File, id_info: BlockId) -> Result<BlockMagic, Box<dyn std::error::Error>>
{
    for magic in id_info.magics {
        let b_offset: u64 = magic.b_offset;
        let magic_len: usize = magic.len.try_into().unwrap(); // FIX

        let mut raw_clone = raw.try_clone()?;
        raw_clone.seek(SeekFrom::Start(b_offset))?;

        let mut buffer = vec![0; magic_len];

        raw_clone.read_exact(&mut buffer)?;

        //println!("Buffer: {:X?}", buffer);

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
