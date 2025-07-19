#![allow(clippy::needless_return)]

#[cfg(test)]
mod tests;

pub(crate) mod checksum;
pub(crate) mod ioctl;
mod util;

pub mod containers;
pub mod partitions;
pub mod filesystems;

use std::{
    fmt::{self, Debug},
    fs::File,
    io::{ErrorKind, Read, Seek, SeekFrom, BufReader},
    os::fd::AsFd,
    path::{Path, PathBuf},
    io::Error as IoError,
};

use bitflags::bitflags;
use uuid::Uuid;
use zerocopy::FromBytes;
use rustix::fs::{fstat, ioctl_blksszget, major, minor, Dev, FileType, Mode};
use crate::{ioctl::{ioctl_blkgetsize64, ioctl_ioc_opal_get_status, 
    OpalStatusFlags}};

use crate::{
    containers::{
        ContError, 
        luks::{LUKS1_ID_INFO, LUKS2_ID_INFO}
    }, 
    partitions::{
        PtError, 
        dos::DOS_PT_ID_INFO,
        gpt::GPT_PT_ID_INFO
    },
    filesystems::{
        FsError,
        exfat::EXFAT_ID_INFO,
        ext::{EXT2_ID_INFO, EXT3_ID_INFO, EXT4_ID_INFO},
        linux_swap::{LINUX_SWAP_V0_ID_INFO, LINUX_SWAP_V1_ID_INFO}, 
        ntfs::NTFS_ID_INFO,
        vfat::VFAT_ID_INFO,
        volume_id::{VolumeId32, VolumeId64},
    }, 
};

#[derive(Debug)]
pub enum BlockidError {
    ArgumentError(&'static str),
    ProbeError(&'static str),
    FsError(FsError),
    PtError(PtError),
    ContError(ContError),
    IoError(IoError),
    NixError(rustix::io::Errno),
}

impl fmt::Display for BlockidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockidError::ArgumentError(e) => write!(f, "Invalid Arguments given: {e}"),
            BlockidError::ProbeError(e) => write!(f, "Probe failed: {e}"),
            BlockidError::FsError(e) => write!(f, "Filesystem probe failed: {e}"),
            BlockidError::PtError(e) => write!(f, "Partition Table probe failed: {e}"),
            BlockidError::ContError(e) => write!(f, "Container probe failed: {e}"),
            BlockidError::IoError(e) => write!(f, "I/O operation failed: {e}"),
            BlockidError::NixError(e) => write!(f, "*Nix operation failed: {e}"),
        }
    }
}

impl std::error::Error for BlockidError {}

impl From<IoError> for BlockidError {
    fn from(err: IoError) -> Self {
        BlockidError::IoError(err)
    }
}

impl From<rustix::io::Errno> for BlockidError {
    fn from(err: rustix::io::Errno) -> Self {
        BlockidError::NixError(err)
    }
}

static PROBES: &[(ProbeFilter, ProbeFilter, BlockidIdinfo)] = &[
    (ProbeFilter::SKIP_CONT, ProbeFilter::SKIP_LUKS1, LUKS1_ID_INFO),
    (ProbeFilter::SKIP_CONT, ProbeFilter::SKIP_LUKS2, LUKS2_ID_INFO),

    (ProbeFilter::SKIP_PT, ProbeFilter::SKIP_DOS, DOS_PT_ID_INFO),
    (ProbeFilter::SKIP_PT, ProbeFilter::SKIP_GPT, GPT_PT_ID_INFO),

    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_EXFAT, EXFAT_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_EXT2, EXT2_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_EXT3, EXT3_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_EXT4, EXT4_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_LINUX_SWAP_V0, LINUX_SWAP_V0_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_LINUX_SWAP_V1, LINUX_SWAP_V1_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_NTFS, NTFS_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_VFAT, VFAT_ID_INFO),
];

impl BlockidProbe {
    pub fn new(
            file: File,
            offset: u64,
            flags: ProbeFlags,
            filter: ProbeFilter,
        ) -> Result<BlockidProbe, BlockidError>
    {   
        let stat = fstat(file.as_fd())?;

        let sector_size: u64 = if FileType::from_raw_mode(stat.st_mode).is_block_device() {
            u64::from(ioctl_blksszget(file.as_fd())?)
        } else {
            512
        };
        
        let size: u64 = if FileType::from_raw_mode(stat.st_mode).is_block_device() {
            ioctl_blkgetsize64(file.as_fd())?
        } else {
            stat.st_size as u64
        };

        //let buffer = BufReader::with_capacity(stat.st_blksize as usize, file.try_clone()?);

        Ok( Self { 
            file,
            buffer: None,
            offset, 
            size, 
            #[cfg(target_arch = "aarch64")]
            io_size: i64::from(stat.st_blksize), 
            #[cfg(target_arch = "x86_64")]
            io_size: stat.st_blksize,
            devno: stat.st_rdev, 
            disk_devno: stat.st_dev,
            sector_size, 
            mode: Mode::from(stat.st_mode), 
            flags,
            filter,
            values: None 
        })
    }

    // Need to figure out how to use buffering when available so this does nothing
    pub fn enable_buffering(&mut self, capacity: usize) -> Result<(), BlockidError> {
        let clone = self.file.try_clone()?;
        self.buffer = Some(BufReader::with_capacity(capacity, clone));
        return Ok(());
    }

    pub fn probe_values(
            &mut self
        ) -> Result<(), BlockidError>
    {
        if self.filter.is_empty() {
            for info in PROBES {
                let result = match probe_get_magic(&mut self.file, &info.2) {
                    Ok(magic) => {
                        match magic {
                            Some(t) => (info.2.probe_fn)(self, t),
                            None => (info.2.probe_fn)(self, BlockidMagic::EMPTY_MAGIC)
                        }
                    },
                    Err(e) => {
                        log::error!("Wrong Magic\nInfo: \"{:?}\",\nError: {:?}", info.2, e);
                        continue
                    },
                };

                if result.is_ok() {
                    return Ok(());
                } else {
                    log::error!("Wrong FS: {:?}", result.unwrap_err());
                }
            }
            return Err(BlockidError::ProbeError("All probe functions exhasted"));
        }
        
        let filtered_probe: Vec<BlockidIdinfo> = PROBES
            .iter()
            .filter_map(|&(catagory, item, id_info)| {
                if !self.filter.contains(catagory) && !self.filter.contains(item) {
                    return Some(id_info);
                } else {
                    return None;
                }
            })
            .collect();
        
        for info in filtered_probe {
            let result = match probe_get_magic(&mut self.file, &info) {
                Ok(magic) => {
                    match magic {
                        Some(t) => (info.probe_fn)(self, t),
                        None => (info.probe_fn)(self, BlockidMagic::EMPTY_MAGIC)
                    }
                },
                Err(_) => continue,
            };
            
            if result.is_ok() {
                return Ok(());
            }
        }

        return Err(BlockidError::ProbeError("All probe filtered functions exhasted"));
    }

    pub(crate) fn push_result(
            &mut self,
            result: ProbeResult,
        ) 
    {
        self.values
            .get_or_insert_with(Vec::new)
            .push(result)
    }

    pub fn probe_from_filename<P: AsRef<Path>>(
            filename: P,
            flags: ProbeFlags,
            filter: ProbeFilter,
            offset: u64,
        ) -> Result<BlockidProbe, BlockidError>
    {
        let file = File::open(filename)?;

        let probe = BlockidProbe::new(file, offset, flags, filter)?;

        return Ok(probe);
    }

    pub fn results(&self) -> Option<&[ProbeResult]> {
        self.values.as_deref()
    }

    pub fn into_results(self) -> Option<Vec<ProbeResult>> {
        self.values
    }

    #[inline]
    pub fn devno(&self) -> Dev {
        self.devno
    }
    
    #[inline]
    pub fn devno_maj(&self) -> u32 {
        major(self.devno)
    }

    #[inline]
    pub fn devno_min(&self) -> u32 {
        minor(self.devno)
    }

    #[inline]
    pub fn disk_devno(&self) -> Dev {
        self.disk_devno
    }

    #[inline]
    pub fn disk_devno_maj(&self) -> u32 {
        major(self.disk_devno)
    }

    #[inline]
    pub fn disk_devno_min(&self) -> u32 {
        minor(self.disk_devno)
    }

    #[inline]
    pub fn is_block_device(&self) -> bool {
        FileType::from_raw_mode(self.mode.as_raw_mode()).is_block_device()
    }

    #[inline]
    pub fn is_regular_file(&self) -> bool {
        FileType::from_raw_mode(self.mode.as_raw_mode()).is_file()
    }

    fn is_opal_locked(
            &mut self
        ) -> Result<bool, rustix::io::Errno>
    {
        if !self.flags.contains(ProbeFlags::OPAL_CHECKED) {
            let status = ioctl_ioc_opal_get_status(self.file.as_fd())?;
        
            if status.flags.contains(OpalStatusFlags::OPAL_FL_LOCKED) {
                self.flags.insert(ProbeFlags::OPAL_LOCKED);
            }
        
            self.flags.insert(ProbeFlags::OPAL_CHECKED);
        }
    
        Ok(self.flags.contains(ProbeFlags::OPAL_LOCKED))
    }
}

#[derive(Debug)]
pub struct BlockidProbe {
    file: File,
    buffer: Option<BufReader<File>>,
    offset: u64,
    size: u64,
    pub io_size: i64, 

    devno: Dev,
    disk_devno: Dev,
    sector_size: u64,
    mode: Mode,
    //pub zone_size: u64,

    flags: ProbeFlags,
    filter: ProbeFilter,
    values: Option<Vec<ProbeResult>>
}

impl BlockidProbeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.path = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn with_offset(mut self, offset: u64) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_flags(mut self, flags: ProbeFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn with_filter(mut self, filter: ProbeFilter) -> Self {
        self.filter = filter;
        self
    }

    pub fn build(self) -> Result<BlockidProbe, BlockidError> {
        let path = self.path.ok_or_else(|| {
            BlockidError::ArgumentError("Path not set in BlockidProbeBuilder")
        })?;

        let file = File::open(&path)?;
        BlockidProbe::new(file, self.offset, self.flags, self.filter)
    }
}

#[derive(Debug, Default)]
pub struct BlockidProbeBuilder {
    path: Option<PathBuf>,
    offset: u64,
    flags: ProbeFlags,
    filter: ProbeFilter,
}

bitflags!{
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct ProbeFlags: u64 {
        const TINY_DEV = 1 << 0;
        const OPAL_CHECKED = 1 << 1;
        const OPAL_LOCKED = 1 << 2;
        const FORCE_GPT_PMBR = 1 << 3;
    }

    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct ProbeFilter: u64 {
        const SKIP_CONT = 1 << 0;
        const SKIP_PT = 1 << 1;
        const SKIP_FS = 1 << 2;
        const SKIP_LUKS1 = 1 << 3;
        const SKIP_LUKS2 = 1 << 4;
        const SKIP_DOS = 1 << 5;
        const SKIP_GPT = 1 << 6;
        const SKIP_EXFAT = 1 << 7;
        const SKIP_EXT2 = 1 << 8;
        const SKIP_EXT3 = 1 << 9;
        const SKIP_EXT4 = 1 << 10;
        const SKIP_LINUX_SWAP_V0 = 1 << 11;
        const SKIP_LINUX_SWAP_V1 = 1 << 12;
        const SKIP_NTFS = 1 << 13;
        const SKIP_VFAT = 1 << 14;
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Endianness {
    Little,
    Big
}

#[derive(Debug)]
pub enum ProbeResult {
    Container(ContainerResults),
    PartTable(PartTableResults),
    Filesystem(FilesystemResults),
}

#[derive(Debug)]
pub struct ContainerResults {
    pub cont_type: Option<ContType>,
    pub label: Option<String>,
    pub cont_uuid: Option<BlockidUUID>,
    pub cont_creator: Option<String>,
    pub usage: Option<UsageType>,
    pub version: Option<BlockidVersion>,
    pub sbmagic: Option<&'static [u8]>,
    pub sbmagic_offset: Option<u64>,
    pub cont_size: Option<u64>,
    pub cont_block_size: Option<u64>,
    pub endianness: Option<Endianness>,
}

#[derive(Debug)]
pub struct PartTableResults {
    pub offset: Option<u64>,

    pub pt_type: Option<PtType>,
    pub pt_uuid: Option<BlockidUUID>,
    pub sbmagic: Option<&'static [u8]>,
    pub sbmagic_offset: Option<u64>,

    pub partitions: Option<Vec<PartitionResults>>,
}

#[derive(Debug, Clone)]
pub struct PartitionResults {
    pub offset: Option<u64>,
    pub size: Option<u64>,

    pub partno: Option<u64>,
    pub part_uuid: Option<BlockidUUID>,
    pub name: Option<String>,

    pub entry_type: Option<PartEntryType>,
    pub entry_attributes: Option<PartEntryAttributes>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PartEntryType {
    Byte(u8),
    Uuid(Uuid),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PartEntryAttributes {
    Mbr(u8),
    Gpt(u64)
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FilesystemResults {
    pub fs_type: Option<FsType>,
    pub sec_type: Option<FsSecType>,
    pub label: Option<String>,
    pub fs_uuid: Option<BlockidUUID>,
    pub log_uuid: Option<BlockidUUID>,
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
    pub endianness: Option<Endianness>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ContType {
    MD,
    LVM,
    DM,
    LUKS1,
    LUKS2,
    Other(&'static str)
}

impl fmt::Display for ContType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MD => write!(f, "MD"),
            Self::LVM => write!(f, "LVM"),
            Self::DM => write!(f, "DM"),
            Self::LUKS1 => write!(f, "LUKS1"),
            Self::LUKS2 => write!(f, "LUKS2"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PtType {
    Dos,
    Gpt,
    Mac,
    Bsd,
}

impl fmt::Display for PtType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dos => write!(f, "Dos"),
            Self::Gpt => write!(f, "Gpt"),
            Self::Mac => write!(f, "Mac"),
            Self::Bsd => write!(f, "Bsd"),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum FsType {
    Exfat,
    Ext2,
    Ext3,
    Ext4,
    LinuxSwap,
    Ntfs,
    Vfat,
}

impl fmt::Display for FsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exfat => write!(f, "Exfat"),
            Self::Ext2 => write!(f, "Ext2"),
            Self::Ext3 => write!(f, "Ext3"),
            Self::Ext4 => write!(f, "Ext4"),
            Self::Ntfs => write!(f, "Ntfs"),
            Self::LinuxSwap => write!(f, "Linux Swap"),
            Self::Vfat => write!(f, "Vfat"),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum FsSecType {
    Fat12,
    Fat16,
    Fat32,
}

impl fmt::Display for FsSecType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fat12 => write!(f, "Fat12"),
            Self::Fat16 => write!(f, "Fat16"),
            Self::Fat32 => write!(f, "Fat32"),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BlockidUUID {
    Uuid(Uuid),
    VolumeId32(VolumeId32),
    VolumeId64(VolumeId64),
}

impl fmt::Display for BlockidUUID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uuid(t) => write!(f, "{t}"),
            Self::VolumeId32(t) => write!(f, "{t}"),
            Self::VolumeId64(t) => write!(f, "{t}"),
        }
    }
}

#[derive(Debug, Copy, Clone, Hash)]
pub struct BlockidIdinfo {
    pub name: Option<&'static str>,
    pub usage: Option<UsageType>,
    pub minsz: Option<u64>,
    pub probe_fn: ProbeFn,
    pub magics: Option<&'static [BlockidMagic]>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum UsageType {
    Filesystem,
    PartitionTable,
    Raid,
    Crypto,
    Other(&'static str),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BlockidVersion {
    String(String),
    Number(u64),
    DevT(Dev),
}

type ProbeFn = fn(&mut BlockidProbe, BlockidMagic) -> Result<(), BlockidError>;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BlockidMagic {
    pub magic: &'static [u8],
    pub len: usize,
    pub b_offset: u64,
}

impl BlockidMagic {
    pub const EMPTY_MAGIC: BlockidMagic = BlockidMagic{magic: &[0], len: 0, b_offset: 0};
}

fn from_file<T: FromBytes, R: Read+Seek>(
        file: &mut R,
        offset: u64,
    ) -> Result<T, IoError> 
{
    let mut buffer = vec![0u8; core::mem::size_of::<T>()];
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut buffer)?;

    let data = T::read_from_bytes(&buffer)
        .map_err(|_| ErrorKind::UnexpectedEof)?;
    
    return Ok(data);
}

fn read_exact_at<const S: usize, R: Read+Seek>(
        file: &mut R,
        offset: u64,
    ) -> Result<[u8; S], IoError>
{
    let mut buffer = [0u8; S];
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut buffer)?;

    return Ok(buffer);
}

fn read_vec_at<R: Read+Seek>(
        file: &mut R,
        offset: u64,
        buf_size: usize
    ) -> Result<Vec<u8>, IoError>
{
    let mut buffer = vec![0u8; buf_size];
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut buffer)?;

    return Ok(buffer);
}

fn read_sector_at<R: Read+Seek>(
        file: &mut R,
        sector: u64,
    ) -> Result<[u8; 512], IoError>
{
    return read_exact_at::<512, R>(file, sector << 9);
}

fn probe_get_magic<R: Read+Seek>(
        file: &mut R, 
        id_info: &BlockidIdinfo
    ) -> Result<Option<BlockidMagic>, IoError>
{
    match id_info.magics {
        Some(magics) => {
            for magic in magics {
                file.seek(SeekFrom::Start(magic.b_offset))?;

                let mut buffer = vec![0; magic.len];

                file.read_exact(&mut buffer)?;

                if buffer == magic.magic {
                    return Ok(Some(*magic));
                }
            }
        },
        None => {
            return Ok(None);
        },
    }

    return Err(ErrorKind::NotFound.into());
}
