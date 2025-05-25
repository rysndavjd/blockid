use crate::volume_id;
use crate::vfat::{VfatExtras, VfatVersion};

use uuid::Uuid;
use std::fs::File;
use nix::sys::stat::dev_t;
use bitflags::bitflags;
use bytemuck::{from_bytes, Pod};
use std::io::{Read, Seek, SeekFrom};

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
    //VolumeId64([u8; 8]),
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
    pub devno: dev_t,
    pub disk_devno: dev_t,
    pub probe_flags: ProbeFlags,
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
            devno: dev_t, 
            disk_devno: dev_t,
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

    pub fn is_tiny(
            &self
        ) -> bool 
    {
        self.probe_flags.contains(ProbeFlags::TINY_DEV)
    }

    pub fn set_fs_type(
            &mut self, 
            fs_type: FsType
        ) 
    {
        self.values.fs.fs_type = Some(fs_type)
    }

    pub fn set_fs_version(
            &mut self, 
            fs_version: FsVersion
        ) 
    {
        self.values.fs.fs_version = Some(fs_version)
    }

    pub fn set_uuid(
            &mut self, 
            uuid: BlkUuid
        ) 
    {
        self.values.fs.uuid = Some(uuid)
    }

    pub fn set_uuid_sub(&mut self, 
            uuid_sub: BlkUuid
        ) 
    {
        self.values.fs.uuid_sub = Some(uuid_sub)
    }

    pub fn set_label_utf8_lossy(&mut self, 
            label: &[u8]
        ) 
    {
        self.values.fs.label = Some(String::from_utf8_lossy(label).to_string())
    }

    pub fn set_usage(
            &mut self, 
            usage: Usage
        ) 
    {
        self.values.fs.usage = Some(usage)
    }

    pub fn set_fs_extras(
            &mut self,
            extra: FsExtras
        )
    {
        self.values.fs.fs_extras = Some(extra)
    }

    pub fn set_fs_block_size (
            &mut self,
            fs_block_size: u64,
        )
    {
        self.values.fs.fs_block_size = Some(fs_block_size)
    }

    pub fn set_block_size (
            &mut self,
            block_size: u64,
        )
    {
        self.values.fs.block_size = Some(block_size)
    }

    pub fn set_fs_size (
            &mut self,
            fs_size: u64,
        )
    {
        self.values.fs.fs_size = Some(fs_size)
    }

    pub fn set_sec_type (
            &mut self,
            sec_type: FsSecType,
        )
    {
        self.values.sec_type = Some(sec_type)
    }
}

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

