mod crc32c;

pub mod partitions;
pub mod filesystems;

use std::os::fd::OwnedFd;
use std::os::unix::fs::MetadataExt;
use std::{fs::File, os::fd::AsFd};
use std::path::Path;
use filesystems::volume_id::{VolumeId32, VolumeId64};
use uuid::Uuid;
use bytemuck::{from_bytes, Pod};
use std::io::{Read, Seek, SeekFrom};
use rustix::fs::{Stat, ioctl_blksszget, Dev, Mode};
use rustix::fs::{fstat, stat};
use crate::filesystems::ext::{EXT2_ID_INFO, EXT3_ID_INFO, EXT4_ID_INFO, EXT4DEV_ID_INFO};
use crate::filesystems::vfat::VFAT_ID_INFO;

static PROBES: &[BlockidIdinfo] = &[
    
    //Filesystems
    #[cfg(feature = "vfat")]
    VFAT_ID_INFO,
    #[cfg(feature = "ext")]
    EXT2_ID_INFO,
    #[cfg(feature = "ext")]
    EXT3_ID_INFO,
    #[cfg(feature = "ext")]
    EXT4_ID_INFO,
    #[cfg(feature = "ext")]
    EXT4DEV_ID_INFO,
];

impl BlockidProbe {
    pub fn new(
            file: &File,
            offset: u64,
            size: u64,
        ) -> Result<BlockidProbe, Box<dyn std::error::Error>>
    {   
        let stat = fstat(&file.as_fd())?;

        Ok( Self { 
            file: file.try_clone()?, 
            offset: offset, 
            size: size, 
            io_size: stat.st_blksize, 
            devno: stat.st_rdev, 
            disk_devno: stat.st_dev, 
            sector_size: ioctl_blksszget(&file.as_fd())?.into(), 
            mode: stat.st_mode.into(), 
            values: None 
        })
    }

    pub fn get_values(
            &mut self
        ) -> Result<(), Box<dyn std::error::Error>>
    {
        for info in PROBES {
            let magic = probe_get_magic(self, info)?;
            let test = (info.probe_fn)(self, magic)?;
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
            filename: &Path
        ) -> Result<BlockidProbe, Box<dyn std::error::Error>>
    {
        let file = File::open(filename)?;
        let probe = BlockidProbe::new(&file, 0, file.metadata()?.size())?;

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
    //pub zone_size: u64, 

    pub values: Option<Vec<ProbeResult>>
}

#[derive(Debug)]
pub enum ProbeResult {
    Container(ContainerResults),
    Filesystem(FilesystemResults),
} 

#[derive(Debug)]
pub struct ContainerResults {
    pub pt_type: Option<String>,
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

#[derive(Debug)]
pub enum FsSecType {
    #[cfg(feature = "vfat")]
    Msdos,
    #[cfg(feature = "ext")]
    Ext2,
    Other(String)
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
    DevId(Dev),
}

pub type ProbeFn = fn(&mut BlockidProbe, BlockidMagic) -> Result<Option<ProbeResult>, Box<dyn std::error::Error>>;

#[derive(Debug, Clone, Copy)]
pub struct BlockidMagic {
    pub magic: &'static [u8],
    pub len: usize,
    pub b_offset: u64,
}

pub fn read_buffer<const BUF_SIZE: usize>(
        probe: &mut BlockidProbe,
        offset: u64,
    ) -> Result<[u8; BUF_SIZE], Box<dyn std::error::Error>> 
{
    let mut block = probe.file.try_clone()?;
    block.seek(SeekFrom::Start(0))?;

    let mut buffer = [0u8; BUF_SIZE];
    block.seek(SeekFrom::Start(offset))?;
    block.read_exact(&mut buffer)?;

    return Ok(buffer);
}

pub fn read_buffer_vec(
        probe: &mut BlockidProbe,
        offset: u64,
        buf_size: usize
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> 
{
    let mut block = probe.file.try_clone()?;
    block.seek(SeekFrom::Start(0))?;

    let mut buffer = vec![0u8; buf_size];
    block.seek(SeekFrom::Start(offset))?;
    block.read_exact(&mut buffer)?;

    return Ok(buffer);
}

pub fn read_sector(
        probe: &mut BlockidProbe,
        sector: u64,
    ) -> Result<[u8; 512], Box<dyn std::error::Error>> 
{
    read_buffer::<512>(probe, sector << 9)
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
    ) -> Result<BlockidMagic, Box<dyn std::error::Error>>
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
