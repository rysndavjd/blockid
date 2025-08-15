use std::{
    fmt,
    fs::File, 
    io::{BufReader, Seek, SeekFrom}, 
    path::{Path, PathBuf}
};

use rustix::{
    fs::{fstat, major, minor, Dev, FileType, Mode},
    fd::AsFd
};
use bitflags::bitflags;
use uuid::Uuid;

use crate::ioctl::{logical_block_size, device_size_bytes};
#[cfg(target_os = "linux")]
use crate::ioctl::{OpalStatusFlags, ioctl_ioc_opal_get_status, 
    ioctl_blkgetzonesz};
use crate::util::probe_get_magic;
use crate::BlockidError;

use crate::{
    containers::{
        luks::{LUKS1_ID_INFO, LUKS2_ID_INFO, LUKS_OPAL_ID_INFO}
    }, 
    partitions::{
        dos::DOS_PT_ID_INFO,
        //gpt::GPT_PT_ID_INFO
    },
    filesystems::{
        exfat::EXFAT_ID_INFO,
        ext::{EXT2_ID_INFO, EXT3_ID_INFO, EXT4_ID_INFO},
        linux_swap::{LINUX_SWAP_V0_ID_INFO, LINUX_SWAP_V1_ID_INFO}, 
        ntfs::NTFS_ID_INFO,
        vfat::VFAT_ID_INFO,
        volume_id::{VolumeId32, VolumeId64},
    }, 
};

static PROBES: &[(ProbeFilter, ProbeFilter, BlockidIdinfo)] = &[
    (ProbeFilter::SKIP_CONT, ProbeFilter::SKIP_LUKS1, LUKS1_ID_INFO),
    (ProbeFilter::SKIP_CONT, ProbeFilter::SKIP_LUKS2, LUKS2_ID_INFO),
    (ProbeFilter::SKIP_CONT, ProbeFilter::SKIP_LUKS_OPAL, LUKS_OPAL_ID_INFO),

    (ProbeFilter::SKIP_PT, ProbeFilter::SKIP_DOS, DOS_PT_ID_INFO),
    //(ProbeFilter::SKIP_PT, ProbeFilter::SKIP_GPT, GPT_PT_ID_INFO),

    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_EXFAT, EXFAT_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_EXT2, EXT2_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_EXT3, EXT3_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_EXT4, EXT4_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_LINUX_SWAP_V0, LINUX_SWAP_V0_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_LINUX_SWAP_V1, LINUX_SWAP_V1_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_NTFS, NTFS_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_VFAT, VFAT_ID_INFO),
];

#[derive(Debug)]
pub struct Probe {
    file: File,
    path: PathBuf,
    buffer: Option<BufReader<File>>,
    offset: u64,
    size: u64,
    io_size: i64,

    devno: Dev,
    disk_devno: Dev,
    sector_size: u64,
    mode: Mode,
    #[cfg(target_os = "linux")]
    zone_size: u64,

    probe_mode: ProbeMode,
    flags: ProbeFlags,
    filter: ProbeFilter,
    value: Option<ProbeResult>
}

impl Probe {
    pub fn supported_string() -> Vec<&'static str> {
        PROBES
            .iter()
            .filter_map(|(_, _, info)| info.name)
            .collect()
    }

    pub fn supported_type() -> Vec<BlockType> {
        PROBES
            .iter()
            .filter_map(|(_, _, info)| info.btype)
            .collect()
    }

    pub fn new(
            file: File,
            path: &Path,
            offset: u64,
            probe_mode: ProbeMode,
            flags: ProbeFlags,
            filter: ProbeFilter,
        ) -> Result<Probe, BlockidError>
    {   
        let stat = fstat(file.as_fd())?;

        let sector_size: u64 = if FileType::from_raw_mode(stat.st_mode).is_block_device() {
            u64::from(logical_block_size(file.as_fd())?)
        } else {
            512
        };
        
        let size: u64 = if FileType::from_raw_mode(stat.st_mode).is_block_device() {
            device_size_bytes(file.as_fd())?
        } else {
            stat.st_size as u64
        };

        //let buffer = BufReader::with_capacity(stat.st_blksize as usize, file.try_clone()?);

        #[cfg(target_os = "linux")]
        let zone_size = u64::from(ioctl_blkgetzonesz(file.as_fd())? << 9);

        Ok( Self { 
            file,
            path: path.to_path_buf(),
            buffer: None,
            offset, 
            size, 
            io_size: stat.st_blksize.into(),
            devno: stat.st_rdev,
            disk_devno: stat.st_dev,
            sector_size, 
            mode: Mode::from(stat.st_mode),
            #[cfg(target_os = "linux")]
            zone_size,
            probe_mode,
            flags,
            filter,
            value: None,
        })
    }

    // Need to figure out how to use buffering when available so this does nothing
    pub fn enable_buffering_with_capacity(&mut self, capacity: usize) -> Result<(), BlockidError> {
        let clone = self.file.try_clone()?;
        self.buffer = Some(BufReader::with_capacity(capacity, clone));
        return Ok(());
    }

    pub fn enable_buffering(&mut self) -> Result<(), BlockidError> {
        self.enable_buffering_with_capacity(self.io_size as usize)?;
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
                        self.file().seek(SeekFrom::Start(0))?;
                        match magic {
                            Some(t) => (info.2.probe_fn)(self, t),
                            None => {
                                (info.2.probe_fn)(self, BlockidMagic::EMPTY_MAGIC)
                            }
                        }
                    },
                    Err(e) => {
                        log::error!("Wrong Magic\nInfo: \"{:?}\",\nError: {:?}", info.2, e);
                        continue
                    },
                };

                if result.is_ok() {
                    return Ok(());
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
        if self.value.is_some() {
            log::error!("Probe already has a result, first: {:?}, second: {result:?}", self.value)
        }
        self.value = Some(result)
    }

    pub fn from_filename(
            filename: &Path,
            probe_mode: ProbeMode,
            flags: ProbeFlags,
            filter: ProbeFilter,
            offset: u64,
        ) -> Result<Probe, BlockidError>
    {
        let file = File::open(&filename)?;
                
        let probe = Probe::new(file, filename, offset, probe_mode, flags, filter)?;

        return Ok(probe);
    }

    pub fn result(&self) -> Option<&ProbeResult> {
        return self.value.as_ref();
    }

    pub fn into_result(self) -> Option<ProbeResult> {
        return self.value;
    }

    #[inline]
    pub fn path(&self) -> &Path {
        return self.path.as_path();
    }
    
    #[inline]
    pub fn size(&self) -> u64 {
        return self.size;
    }

    #[inline]
    pub fn offset(&self) -> u64 {
        return self.offset;
    }

    #[inline]
    pub fn ssz(&self) -> u64 {
        return self.sector_size;
    }

    #[cfg(target_os = "linux")]
    #[inline]
    pub fn zsz(&self) -> u64 {
        return self.zone_size;
    }

    #[inline]
    pub fn devno(&self) -> Dev {
        return self.devno;
    }
    
    #[inline]
    pub fn devno_maj(&self) -> u32 {
        return major(self.devno);
    }

    #[inline]
    pub fn devno_min(&self) -> u32 {
        return minor(self.devno);
    }

    #[inline]
    pub fn disk_devno(&self) -> Dev {
        return self.disk_devno;
    }

    #[inline]
    pub fn disk_devno_maj(&self) -> u32 {
        return major(self.disk_devno);
    }

    #[inline]
    pub fn disk_devno_min(&self) -> u32 {
        return minor(self.disk_devno);
    }

    #[inline]
    pub fn is_block_device(&self) -> bool {
        return FileType::from_raw_mode(self.mode.as_raw_mode())
            .is_block_device();
    }

    #[inline]
    pub fn is_regular_file(&self) -> bool {
        return FileType::from_raw_mode(self.mode.as_raw_mode())
            .is_file();
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn is_opal_locked(
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
    
        return Ok(self.flags.contains(ProbeFlags::OPAL_LOCKED));
    }

    pub fn filters(&self) -> ProbeFilter {
        self.filter
    }

    pub fn flags(&self) -> ProbeFlags {
       self.flags 
    }

    pub fn file(&self) -> &File {
        &self.file
    }

}


#[derive(Debug, Default, Clone, Copy)]
pub enum ProbeMode {
    #[default]
    Single,
    Recursive
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
        const SKIP_LUKS_OPAL = 1 << 5;
        const SKIP_DOS = 1 << 6;
        const SKIP_GPT = 1 << 7;
        const SKIP_EXFAT = 1 << 8;
        const SKIP_EXT2 = 1 << 9;
        const SKIP_EXT3 = 1 << 10;
        const SKIP_EXT4 = 1 << 11;
        const SKIP_LINUX_SWAP_V0 = 1 << 12;
        const SKIP_LINUX_SWAP_V1 = 1 << 13;
        const SKIP_NTFS = 1 << 14;
        const SKIP_VFAT = 1 << 15;
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ProbeResult {
    Container(ContainerResult),
    PartTable(PartTableResult),
    Filesystem(FilesystemResult),
    List(Vec<ProbeResult>)
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ContainerResult {
    pub btype: Option<BlockType>,
    pub sec_type: Option<SecType>,
    pub uuid: Option<BlockidUUID>,
    pub label: Option<String>,
    pub creator: Option<String>,
    pub usage: Option<UsageType>,
    pub version: Option<BlockidVersion>,
    pub sbmagic: Option<&'static [u8]>,
    pub sbmagic_offset: Option<u64>,
    pub endianness: Option<Endianness>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PartTableResult {
    pub btype: Option<BlockType>,
    pub sec_type: Option<SecType>,
    pub uuid: Option<BlockidUUID>,
    pub creator: Option<String>,
    pub usage: Option<UsageType>,
    pub version: Option<BlockidVersion>,
    pub partitions: Option<Vec<PartitionResults>>,
    pub sbmagic: Option<&'static [u8]>,
    pub sbmagic_offset: Option<u64>,
    pub endianness: Option<Endianness>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FilesystemResult {
    pub btype: Option<BlockType>,
    pub sec_type: Option<SecType>,
    pub uuid: Option<BlockidUUID>,
    pub log_uuid: Option<BlockidUUID>,
    pub ext_journal: Option<BlockidUUID>, 
    pub label: Option<String>,
    pub creator: Option<String>,
    pub usage: Option<UsageType>,
    pub size: Option<u64>,
    pub fs_last_block: Option<u64>,
    pub fs_block_size: Option<u64>,
    pub block_size: Option<u64>,
    pub version: Option<BlockidVersion>,
    pub sbmagic: Option<&'static [u8]>,
    pub sbmagic_offset: Option<u64>,
    pub endianness: Option<Endianness>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BlockType {
    LUKS1,
    LUKS2,
    LUKSOPAL,
    Dos,
    Gpt,
    Exfat,
    Jbd,
    Ext2,
    Ext3,
    Ext4,
    LinuxSwapV0,
    LinuxSwapV1,
    SwapSuspend,
    Ntfs,
    Vfat,
}

impl fmt::Display for BlockType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LUKS1 => write!(f, "LUKS1"),
            Self::LUKS2 => write!(f, "LUKS2"),
            Self::LUKSOPAL => write!(f, "LUKS Opal"),
            Self::Dos => write!(f, "Dos"),
            Self::Gpt => write!(f, "Gpt"),
            Self::Exfat => write!(f, "Exfat"),
            Self::Jbd => write!(f, "Jbd"),
            Self::Ext2 => write!(f, "Ext2"),
            Self::Ext3 => write!(f, "Ext3"),
            Self::Ext4 => write!(f, "Ext4"),
            Self::Ntfs => write!(f, "Ntfs"),
            Self::LinuxSwapV0 => write!(f, "Linux Swap V0"),
            Self::LinuxSwapV1 => write!(f, "Linux Swap V1"),
            Self::SwapSuspend => write!(f, "Swap Suspend"),
            Self::Vfat => write!(f, "Vfat"),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SecType {
    Fat12,
    Fat16,
    Fat32,
}

impl fmt::Display for SecType {
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
    pub btype: Option<BlockType>,
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BlockidVersion {
    Number(u64),
    DevT(Dev),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Endianness {
    Little,
    Big
}

type ProbeFn = fn(&mut Probe, BlockidMagic) -> Result<(), BlockidError>;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BlockidMagic {
    pub magic: &'static [u8],
    pub len: usize,
    pub b_offset: u64,
}

impl BlockidMagic {
    pub const EMPTY_MAGIC: BlockidMagic = BlockidMagic{magic: &[0], len: 0, b_offset: 0};
}
