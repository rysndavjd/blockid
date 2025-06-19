mod checksum;

pub mod partitions;
pub mod filesystems;

use std::fmt;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::{fs::File, os::fd::AsFd, path::Path};
use filesystems::volume_id::{VolumeId32, VolumeId64};
use uuid::Uuid;
use bytemuck::{from_bytes, Pod};
use std::io::{self, BufRead, ErrorKind, Read, Seek, SeekFrom};
use rustix::fs::{ioctl_blksszget, Dev, Mode, fstat};
use rustix::io::Errno;
use thiserror::Error;
use crate::filesystems::FsError;
use crate::partitions::PtError;
use crate::filesystems::ext::{EXT2_ID_INFO, EXT3_ID_INFO, EXT4_ID_INFO};
use crate::filesystems::vfat::VFAT_ID_INFO;
use bitflags::{bitflags, Flags};

#[derive(Error, Debug)]
pub enum BlockidError {
    #[error("Filesystem probe failed")]
    FsError(#[from] FsError),
    #[error("Partition Table probe failed")]
    PtError(#[from] PtError),
    #[error("I/O operation failed")]
    IoError(#[from] io::Error),
    #[error("*Nix operation failed")]
    NixError(#[from] Errno),
}

pub static PROBES: &[BlockidIdinfo] = &[
    
    //Filesystems
    #[cfg(feature = "vfat")]
    VFAT_ID_INFO,
    #[cfg(feature = "ext")]
    EXT2_ID_INFO,
    #[cfg(feature = "ext")]
    EXT3_ID_INFO,
    #[cfg(feature = "ext")]
    EXT4_ID_INFO,
];

impl BlockidProbe {
    pub fn new(
            file: &File,
            offset: u64,
            size: u64,
            flags: ProbeFlags,
            filter: ProbeFilter,
        ) -> Result<BlockidProbe, BlockidError>
    {   
        let stat = fstat(file.as_fd())?;
        file.as_raw_fd();
        Ok( Self { 
            file: file.try_clone()?, 
            offset: offset, 
            size, 
            io_size: stat.st_blksize, 
            devno: stat.st_rdev, 
            disk_devno: stat.st_dev, 
            sector_size: ioctl_blksszget(file.as_fd())?.into(), 
            mode: stat.st_mode.into(), 
            flags,
            filter,
            values: None 
        })
    }

    pub fn probe_values(
            &mut self
        ) -> Result<(), BlockidError>
    {
        if self.filter.is_empty() {
            for info in PROBES {
                let magic = probe_get_magic(self, info)?;
                let result = (info.probe_fn)(self, magic)?;
                self.push_result(result);
            }
        }
        
        let mut filtered_probe: BlockidIdinfo;

        if !self.filter.contains(ProbeFilter::SKIP_CONT) {

        } else {
            
        }

        Ok(())
    }

    pub fn push_result(
            &mut self,
            result: ProbeResult,
        ) 
    {
        self.values
            .get_or_insert_with(Vec::new)
            .push(result)
    }

    fn probe_from_filename(
            filename: &Path,
            offset: u64,
            size: u64,
        ) -> Result<BlockidProbe, BlockidError>
    {
        let file = File::open(filename)?;

        let probe = BlockidProbe::new(&file, offset, size, ProbeFlags::empty(), ProbeFilter::empty())?;

        return Ok(probe);
    }

}

#[derive(Debug)]
pub struct BlockidProbe {
    pub file: File,
    pub offset: u64,
    pub size: u64,
    pub io_size: i64, 

    pub devno: Dev,
    pub disk_devno: Dev,
    pub sector_size: u64,
    pub mode: Mode,
    //pub zone_size: u64, //There seems to be no safe function to get zone size so i leave it out

    pub flags: ProbeFlags,
    pub filter: ProbeFilter,
    pub values: Option<Vec<ProbeResult>>
}

bitflags!{
    #[derive(Debug)]
    pub struct ProbeFlags: u32 {
        const TINY_DEV = 0;
    }

    #[derive(Debug)]
    pub struct ProbeFilter: u32 {
        const SKIP_CONT = 0;
        const SKIP_PT = 1;
        const SKIP_FS = 2;
        #[cfg(feature = "vfat")]
        const SKIP_VFAT = 3;
        #[cfg(feature = "ext")]
        const SKIP_EXT = 4; 
    }
}

#[derive(Debug)]
pub enum ProbeResult {
    Container(ContainerResults),       // Raid/Encryption containers
    PartTable(PartTableResults),       // Partition Tables
    Filesystem(FilesystemResults),     // Filesystems
}

#[derive(Debug)]
pub struct ContainerResults {
    pub cont_type: Option<ContType>,
    pub label: Option<String>,
    pub cont_uuid: Option<BlockidUUID>,
    pub cont_creator: Option<String>,
    pub usage: Option<UsageType>,
    pub version: Option<BlockidVersion>,
    pub cont_size: Option<u64>,
    pub cont_block_size: Option<u64>,
}

#[derive(Debug)]
pub struct PartTableResults {
    pub pt_type: Option<PtType>,
    pub pt_uuid: Option<BlockidUUID>,
    pub part_entry_scheme: Option<String>,
    pub part_entry_name: Option<String>,
    pub part_entry_uuid: Option<BlockidUUID>,
    //pub part_entry_type: Option<BlockidPartEntryType>,
    //pub part_entry_flags: Option<String>,
    pub part_entry_number: Option<u64>,
    pub part_entry_offset: Option<u64>,
    pub part_entry_size: Option<u64>,
    pub part_entry_disk: Option<Dev>,
}

#[derive(Debug)]
pub struct FilesystemResults {
    pub fs_type: Option<FsType>,
    pub sec_type: Option<FsSecType>,
    pub label: Option<String>,
    //pub label_raw: Option<BlockidUUID>,
    pub fs_uuid: Option<BlockidUUID>,
    //pub fs_uuid_raw: Option<BlockidUUID>,
    pub log_uuid: Option<BlockidUUID>,
    //pub log_uuid_raw: Option<BlockidUUID>,
    pub ext_journal: Option<BlockidUUID>,
    pub fs_creator: Option<String>,
    pub usage: Option<UsageType>,
    pub version: Option<BlockidVersion>,
    pub sbmagic: Option<&'static [u8]>,
    pub sbmagic_offset: Option<u64>,
    pub fs_size: Option<u64>,
    pub fs_last_block: Option<u64>,
    pub fs_block_size: Option<u64>,
    pub block_size: Option<u64>,
}

#[derive(Debug)]
pub enum ContType {
    #[cfg(feature = "md")]
    Md,
    #[cfg(feature = "lvm")]
    Lvm,
    #[cfg(feature = "dm")]
    Dm,
    Other(String)
}

impl fmt::Display for ContType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "md")]
            Self::Md => write!(f, "Md"),
            #[cfg(feature = "lvm")]
            Self::Lvm => write!(f, "Lvm"),
            #[cfg(feature = "dm")]
            Self::Dm => write!(f, "Dm"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug)]
pub enum PtType {
    #[cfg(feature = "dos")]
    Dos,
    #[cfg(feature = "gpt")]
    Gpt,
    #[cfg(feature = "mac")]
    Mac,
    #[cfg(feature = "bsd")]
    Bsd,
    Other(String)
}

impl fmt::Display for PtType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "dos")]
            Self::Dos => write!(f, "Dos"),
            #[cfg(feature = "gpt")]
            Self::Gpt => write!(f, "Gpt"),
            #[cfg(feature = "mac")]
            Self::Mac => write!(f, "Mac"),
            #[cfg(feature = "bsd")]
            Self::Bsd => write!(f, "Bsd"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug)]
pub enum FsType {
    #[cfg(feature = "vfat")]
    Vfat,
    #[cfg(feature = "ext")]
    Ext2,
    #[cfg(feature = "ext")]
    Ext3,
    #[cfg(feature = "ext")]
    Ext4,
    Other(String)
}

impl fmt::Display for FsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "vfat")]
            Self::Vfat => write!(f, "Vfat"),
            #[cfg(feature = "ext")]
            Self::Ext2 => write!(f, "Ext2"),
            #[cfg(feature = "ext")]
            Self::Ext3 => write!(f, "Ext3"),
            #[cfg(feature = "ext")]
            Self::Ext4 => write!(f, "Ext4"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug)]
pub enum FsSecType {
    #[cfg(feature = "vfat")]
    Fat12,
    #[cfg(feature = "vfat")]
    Fat16,
    #[cfg(feature = "vfat")]
    Fat32,
    Other(String)
}

impl fmt::Display for FsSecType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "vfat")]
            Self::Fat12 => write!(f, "Fat12"),
            #[cfg(feature = "vfat")]
            Self::Fat16 => write!(f, "Fat16"),
            #[cfg(feature = "vfat")]
            Self::Fat32 => write!(f, "Fat32"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug)]
pub enum BlockidUUID {
    Standard(Uuid),
    VolumeId32(VolumeId32),
    VolumeId64(VolumeId64),
    Other(&'static [u8]),
}

#[derive(Debug)]
pub struct BlockidIdinfo {
    pub name: Option<&'static str>,
    pub usage: Option<UsageType>,
    pub minsz: Option<u64>,
    pub probe_fn: ProbeFn,
    pub magics: &'static [BlockidMagic],
}

#[derive(Debug)]
pub enum UsageType {
    Filesystem,
    PartitionTable,
    Raid,
    Crypto,
    Other(&'static str),
}

#[derive(Debug)]
pub enum BlockidVersion {
    String(String),
    Number(u64),
    DevT(Dev),
}

pub type ProbeFn = fn(&mut BlockidProbe, BlockidMagic) -> Result<ProbeResult, BlockidError>;

#[derive(Debug, Clone, Copy)]
pub struct BlockidMagic {
    pub magic: &'static [u8],
    pub len: usize,
    pub b_offset: u64,
}

pub fn read_buffer<const BUF_SIZE: usize, R: Read+Seek>(
        file: &mut R,
        offset: u64,
    ) -> Result<[u8; BUF_SIZE], Box<dyn std::error::Error>> 
{
    file.seek(SeekFrom::Start(0))?;

    let mut buffer = [0u8; BUF_SIZE];
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut buffer)?;

    return Ok(buffer);
}

pub fn read_buffer_vec<R: Read+Seek>(
        file: &mut R,
        offset: u64,
        buf_size: usize
    ) -> Result<Vec<u8>, io::Error> 
{
    file.seek(SeekFrom::Start(0))?;

    let mut buffer = vec![0u8; buf_size];
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut buffer)?;

    return Ok(buffer);
}

pub fn read_sector(
        probe: &mut BlockidProbe,
        sector: u64,
    ) -> Result<[u8; 512], Box<dyn std::error::Error>> 
{
    read_buffer::<512, File>(&mut probe.file, sector << 9)
}

pub fn get_sectorsize(
        probe: &mut BlockidProbe
    ) -> Result<u32, Box<dyn std::error::Error>> 
{
    return Ok(ioctl_blksszget(probe.file.as_fd())?);
}

pub fn probe_get_magic(
        probe: &mut BlockidProbe, 
        id_info: &BlockidIdinfo
    ) -> Result<BlockidMagic, io::Error>
{
    for magic in id_info.magics {
        let b_offset: u64 = magic.b_offset;
        let magic_len: usize = magic.len;

        let mut raw = probe.file.try_clone()?;
        raw.seek(SeekFrom::Start(b_offset))?;

        let mut buffer = vec![0; magic_len];

        raw.read_exact(&mut buffer)?;

        if buffer == magic.magic {
            return Ok(*magic);
        }
    }
    return Err(ErrorKind::NotFound.into());
}

pub fn read_as<T: Pod, R: Read+Seek>(
        raw_block: &mut R,
        offset: u64,
    ) -> Result<T, io::Error> 
{
    raw_block.seek(SeekFrom::Start(0))?;

    let mut buffer = vec![0u8; std::mem::size_of::<T>()];
    raw_block.seek(SeekFrom::Start(offset))?;
    raw_block.read_exact(&mut buffer)?;

    let ptr = from_bytes::<T>(&buffer);
    Ok(*ptr)
}
