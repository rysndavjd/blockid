use std::u16;
use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use byteorder::{ByteOrder, LittleEndian};
use bytemuck::checked::from_bytes;
use bytemuck::{Pod, Zeroable};
use thiserror::Error;
use std::io;

use crate::filesystems::volume_id::VolumeId32;
use crate::{probe_get_magic, read_as, read_buffer_vec, FilesystemResults, FsType};
use crate::{BlockidUUID, FsSecType, BlockidMagic, BlockidIdinfo, UsageType, BlockidProbe, ProbeResult, BlockidError};
use crate::filesystems::FsError;

#[derive(Error, Debug)]
pub enum FatError {
    #[error("I/O operation failed")]
    IoError(#[from] io::Error),
    #[error("Fat Header Error: {0}")]
    FatHeaderError(String),
    #[error("Not an Fat superblock: {0}")]
    UnknownFilesystem(String),
}

impl From<FatError> for FsError {
    fn from(err: FatError) -> Self {
        match err {
            FatError::IoError(e) => FsError::IoError(e),
            FatError::FatHeaderError(info) => FsError::InvalidHeader(info),
            FatError::UnknownFilesystem(info) => FsError::UnknownFilesystem(info),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VfatVersion {
    Fat12,
    Fat16,
    Fat32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VfatExtras {
    oem_name: Option<String>,
    boot_label: Option<String>
}

pub const VFAT_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("vfat"),
    usage: Some(UsageType::Filesystem),
    probe_fn: |probe, magic| {
        probe_vfat(probe, magic)
        .map_err(FsError::from)
        .map_err(BlockidError::from)
    },
    minsz: None,
    magics: &[
        BlockidMagic {
            magic: b"MSWIN",
            len: 5,
            b_offset: 0x52,
        },
        BlockidMagic {
            magic: b"FAT32   ",
            len: 8,
            b_offset: 0x52,
        },
        BlockidMagic {
            magic: b"MSDOS",
            len: 5,
            b_offset: 0x36,
        },
        BlockidMagic {
            magic: b"FAT16   ",
            len: 8,
            b_offset: 0x36,
        },
        BlockidMagic {
            magic: b"FAT12   ",
            len: 8,
            b_offset: 0x36,
        },
        BlockidMagic {
            magic: b"FAT     ",
            len: 8,
            b_offset: 0x36,
        },
        /* I dont know what this is, taken from libblkid so i am not messing with it now
        BlockMagic {
            magic: &[0xEB],
            len: 1,
            b_offset: None,
        },
        BlockMagic {
            magic: &[0xE9],
            len: 1,
            b_offset: None,
        },
        */
        BlockidMagic {
            magic: &[0x55, 0xAA],
            len: 2,
            b_offset: 0x1fe,
        },
    ]
};

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct VFatSuperBlock {
    pub vs_ignored: [u8; 3],
    pub vs_sysid: [u8; 8],
    pub vs_sector_size: u16,
    pub vs_cluster_size: u8,
    pub vs_reserved: u16,
    pub vs_fats: u8,
    pub vs_dir_entries: u16,
    pub vs_sectors: u16, 
    pub vs_media: u8,
    pub vs_fat_length: u16, 
    pub vs_secs_track: u16,
    pub vs_heads: u16,
    pub vs_hidden: u32,
    pub vs_total_sect: u32, 

    pub vs_fat32_length: u32,
    pub vs_flags: u16,
    pub vs_version: u16,
    pub vs_root_cluster: u32,
    pub vs_fsinfo_sector: u16,
    pub vs_backup_boot: u16,
    pub vs_reserved2: [u8; 12],
    pub vs_drive_number: u8,
    pub vs_boot_flags: u8,
    pub vs_ext_boot_sign: u8, /* 0x28 - without vs_label/vs_magic; 0x29 - with */
    pub vs_serno: [u8; 4],
    pub vs_label: [u8; 11],
    pub vs_magic: [u8; 8],
    pub vs_dummy2: [u8; 420],
    pub vs_pmagic: [u8; 2],
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MsDosSuperBlock {
    /* DOS 2.0 BPB */
    pub ms_ignored: [u8; 3],
    pub ms_sysid: [u8; 8],
    pub ms_sector_size: u16,
    pub ms_cluster_size: u8,
    pub ms_reserved: u16,
    pub ms_fats: u8,
    pub ms_dir_entries: u16,
    pub ms_sectors: u16, /* =0 iff V3 or later */
    pub ms_media: u8,
    pub ms_fat_length: u16, /* Sectors per FAT */
    /* DOS 3.0 BPB */
    pub ms_secs_track: u16,
    pub ms_heads: u16,
    pub ms_hidden: u32,
    /* DOS 3.31 BPB */
    pub ms_total_sect: u32,
    /* DOS 3.4 EBPB */
    pub ms_drive_number: u8,
    pub ms_boot_flags: u8,
    pub ms_ext_boot_sign: u8,
    pub ms_serno: [u8; 4],
    /* DOS 4.0 EBPB */
    pub ms_label: [u8; 11],
    pub ms_magic: [u8; 8],
    /* padding */
    pub ms_dummy2: [u8; 448],
    pub ms_pmagic: [u8; 2],
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct VfatDirEntry {
    name: [u8; 11],
    attr: u8,
    time_creat: u16,
    date_creat: u16,
    time_acc: u16,
    date_acc: u16,
    cluster_high: u16,
    time_write: u16,
    date_wriet: u16,
    cluster_low: u16,
    size: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Fat32FsInfo {
    signature1: [u8; 4],
    reserved1: [u8; 120],
    signature2: [u8; 4],
    free_clusters: u32,
    next_cluster: u32,
    reserved2: [u8; 4],
}

const FAT12_MAX: u32 = 0xFF4;
const FAT16_MAX: u32 = 0xFFF4;
const FAT32_MAX: u32 = 0x0FFFFFF6;

const FAT_ATTR_VOLUME_ID: u8 = 0x08;
const FAT_ATTR_DIR: u8 = 0x10;
const FAT_ATTR_LONG_NAME: u8 = 0x0f;
const FAT_ATTR_MASK: u8 = 0x3f;
const FAT_ENTRY_FREE: u8 = 0xe5;

fn is_power_2(num: u64) -> bool {
    return num != 0 && ((num & (num - 1)) == 0); 
}

fn read_vfat_dir_entry(
        raw_block: &File,
        offset: u32,
    ) -> Result<VfatDirEntry, FatError> 
{
    let mut block = raw_block.try_clone()?;
    block.seek(SeekFrom::Start(0))?;

    let mut buffer = [0u8; 32];
    block.seek(SeekFrom::Start(offset.into()))?;
    block.read_exact(&mut buffer)?;

    return Ok(*from_bytes::<VfatDirEntry>(&buffer));
}

pub fn get_fat_size (
        ms: MsDosSuperBlock,
        vs: VFatSuperBlock,
    ) -> u32
{   
    let num_fat: u32 = ms.ms_fats.into();
    let fat_length: u32 = if ms.ms_fat_length == 0 {
        vs.vs_fat32_length
    } else {
        ms.ms_fat_length.into()
    };

    return fat_length * num_fat;
}

fn get_cluster_count (
        ms: MsDosSuperBlock,
        vs: VFatSuperBlock,
    ) -> u32
{
    let sect_count: u32 = if ms.ms_sectors == 0 {
        ms.ms_total_sect
    } else {
        ms.ms_sectors.into()
    };

    let sector_size: u32 = ms.ms_sector_size.into();
    let cluster_count: u32 = (sect_count - (ms.ms_reserved as u32 + get_fat_size(ms, vs) + ((ms.ms_dir_entries as u32 * 32) + (sector_size - 1) / sector_size))) / ms.ms_cluster_size as u32;
    
    return cluster_count;
}

fn get_sect_count (
        ms: MsDosSuperBlock,
    ) -> u32
{
    let sect_count: u32 = if ms.ms_sectors == 0 {
        ms.ms_total_sect
    } else {
        ms.ms_sectors.into()
    };

    return sect_count;
}

fn valid_fat (
        ms: MsDosSuperBlock,
        vs: VFatSuperBlock,
        mag: BlockidMagic,
    ) -> Result<(), FatError> 
{    
    if mag.len <= 2 {
        if ms.ms_pmagic[0] != 0x55 || ms.ms_pmagic[1] != 0xAA {
            return Err(FatError::UnknownFilesystem("Given block is not Fat likely MBR".into()));
        }

        /* Straight From libblkid
		 * OS/2 and apparently DFSee will place a FAT12/16-like
		 * pseudo-superblock in the first 512 bytes of non-FAT
		 * filesystems --- at least JFS and HPFS, and possibly others.
		 * So we explicitly check for those filesystems at the
		 * FAT12/16 filesystem magic field identifier, and if they are
		 * present, we rule this out as a FAT filesystem, despite the
		 * FAT-like pseudo-header.
		 */

        if &ms.ms_magic == b"JFS     " || &ms.ms_magic == b"HPFS    " {
            return Err(FatError::UnknownFilesystem("JFS/HPFS found".into()));
        }
    }

    if ms.ms_fats == 0 {
        return Err(FatError::FatHeaderError("Should be atleast one fat table".into()));
    }
    if ms.ms_reserved == 0 {
        return Err(FatError::FatHeaderError("ms_reserved should not be 0".into()));
    }

    if !is_power_2(ms.ms_cluster_size.into()) {
        return Err(FatError::FatHeaderError("cluster_size is not ^2".into()));
    }

    let cluster_count: u32 = get_cluster_count(ms, vs);

    let max_count = if ms.ms_fat_length == 0 && vs.vs_fat32_length > 0 {
        FAT32_MAX
    } else if cluster_count > FAT12_MAX {
        FAT16_MAX
    } else {
        FAT12_MAX
    };

    if cluster_count > max_count {
        return Err(FatError::FatHeaderError("Too many clusters".into()));
    }
    
    if cluster_count < FAT12_MAX || cluster_count < FAT16_MAX || cluster_count < FAT32_MAX {
        return Ok(())
    } else {
        return Err(FatError::UnknownFilesystem("Unknown fat type".into()));
    }

}

pub fn probe_is_vfat(
        probe: &mut BlockidProbe,
    ) -> Result<Option<ProbeResult>, FatError>
{
    let ms: MsDosSuperBlock = read_as(&probe.file, 0)?;
    let vs: VFatSuperBlock = read_as(&probe.file, 0)?;

    let mag: BlockidMagic = probe_get_magic(probe, &VFAT_ID_INFO)?;
    
    valid_fat(ms, vs, mag)?;

    return Ok(None);
}

pub fn search_fat_label(
        probe: &mut BlockidProbe,
        root_start: u32,
        root_dir_entries: u32,
    ) -> Result<String, FatError> 
{
    let is_tiny = false; // !probe.flags.contains(BlockidFlags::TINY_DEV);

    for i in 0..root_dir_entries {
        let offset = if is_tiny {
            root_start
        } else {
            root_start + (i * 32)
        };
    
        let entry = read_vfat_dir_entry(&probe.file, offset)?;

        if entry.name[0] == 0x00 {
            break;
        }

        if entry.name[0] == FAT_ENTRY_FREE || 
            (entry.cluster_high != 0 || entry.cluster_low != 0) || 
            (entry.attr & FAT_ATTR_MASK) == FAT_ATTR_LONG_NAME 
        {
            continue;
        }

        if (entry.attr & (FAT_ATTR_VOLUME_ID | FAT_ATTR_DIR)) == FAT_ATTR_VOLUME_ID {
            let mut label = entry.name;
            if label[0] == 0x05 {
                label[0] = 0xE5;
            }
            return Ok(String::from_utf8_lossy(&label).to_string());
        }
    }

    return Err(FatError::IoError(io::Error::new(ErrorKind::NotFound, "Fat label not found")));
}

pub fn probe_vfat(
    probe: &mut BlockidProbe,
    mag: BlockidMagic,
) -> Result<ProbeResult, FatError> 
{
    let ms: MsDosSuperBlock = read_as(&probe.file, 0)?;
    let vs: VFatSuperBlock = read_as(&probe.file, 0)?;

    valid_fat(ms, vs, mag)?;

    let cluster_count: u32 = get_cluster_count(ms, vs);
    let fat_size: u32 = get_fat_size(ms, vs);
    let sector_size: u32 = ms.ms_sector_size.into();
    let reserved: u32 = ms.ms_reserved.into();

    if ms.ms_fat_length != 0 {
        let root_start: u32 = (reserved + fat_size) * sector_size;
        let root_dir_entries: u32 = vs.vs_dir_entries.into();

        let vol_label = search_fat_label(probe, root_start, root_dir_entries)?;
        
        let boot_label: Option<String> = if ms.ms_ext_boot_sign == 0x29 {
            Some(String::from_utf8_lossy(&ms.ms_label).to_string())
        } else {
            None
        };

        let vol_serno: VolumeId32 = if ms.ms_ext_boot_sign == 0x28 || ms.ms_ext_boot_sign == 0x29 {
            VolumeId32::new(ms.ms_serno)
        } else { 
            return Err(FatError::FatHeaderError("Unable to get Volumeid".into()));
        };

        if !(cluster_count < FAT12_MAX || cluster_count < FAT16_MAX) {
            return Err(FatError::UnknownFilesystem("Unknown Fat version".into()));
        };
        
        let oem_name = String::from_utf8_lossy(&ms.ms_sysid).to_string();

        //probe.set_fs_type(FsType::Vfat);
        //probe.set_fs_version(FsVersion::Vfat(fat_version));
        //probe.set_uuid(BlkUuid::VolumeId32(vol_serno));
        //probe.set_label_utf8_lossy(&vol_label);
        //probe.set_usage(Usage::Filesystem);
        //probe.set_fs_block_size();
        //probe.set_block_size();
        //probe.set_fs_size( );
        //probe.set_fs_extras(FsExtras::Vfat(VfatExtras { oem_name: Some(oem_name), boot_label: boot_label }));
        //probe.set_sec_type(FsSecType::Msdos);

        return Ok(ProbeResult::Filesystem(FilesystemResults { 
                                    fs_type: Some(FsType::Vfat), 
                                    sec_type: Some(FsSecType::Msdos), 
                                    label: Some(vol_label), 
                                    fs_uuid: Some(BlockidUUID::VolumeId32(vol_serno)), 
                                    log_uuid: None, 
                                    ext_journal: None, 
                                    fs_creator: Some(oem_name), 
                                    usage: Some(UsageType::Filesystem), 
                                    version: None, 
                                    sbmagic: Some(mag.magic), 
                                    sbmagic_offset: Some(mag.b_offset), 
                                    fs_size: Some(sector_size as u64 * get_sect_count(ms) as u64), 
                                    fs_last_block: None, 
                                    fs_block_size: Some(vs.vs_cluster_size as u64 * sector_size as u64), 
                                    block_size: Some(sector_size as u64) 
                                }
                            )
                        );
    } else if vs.vs_fat32_length != 0 {
        let mut maxloop = 100;
        
        let cluster_size: u32 = vs.vs_cluster_size.into(); 
        let buf_size: u32 = cluster_size * sector_size;
        let start_data_sect: u32 = reserved + fat_size;
        let entries: u32 = vs.vs_fat32_length * sector_size / 4;
        let mut next: u32 = vs.vs_root_cluster;

        let mut vol_label: Option<String> = None;

        while next != 0 && next < entries && maxloop > 0 {
            maxloop -= 1;

            let next_sect_off: u32 = (next - 2) * cluster_size;
            let next_off: u32 = (start_data_sect + next_sect_off) * sector_size;
            let count: u32 = buf_size / 32; 

            match search_fat_label(probe, next_off, count) {
                Ok(label) => {
                    vol_label = Some(label);
                    break;
                }
                Err(_) => {
                    let fat_entry_offset: u32 = (reserved * sector_size) + (next * 4);
                    let buffer: Vec<u8> = read_buffer_vec(probe, fat_entry_offset as u64, buf_size as usize)?;
                    
                    if buffer.len() < 4 {
                        break;
                    }
                    next = LittleEndian::read_u32(&buffer[0..4]) & 0x0FFFFFFF;
                }
            }
        };
        let fsinfo_sect = vs.vs_fsinfo_sector;
        
        if fsinfo_sect != 0 {
            let fsinfo = read_as::<Fat32FsInfo>(&probe.file, fsinfo_sect as u64 * sector_size as u64)?;

            if &fsinfo.signature1 != b"\x52\x52\x61\x41" &&
               &fsinfo.signature1 != b"\x52\x52\x64\x41" &&
               &fsinfo.signature1 != b"\x00\x00\x00\x00" 
            {
                return Err(FatError::FatHeaderError("Invalid fsinfo.signature1".into()));
            }

            if &fsinfo.signature2 != b"\x72\x72\x41\x61" &&
               &fsinfo.signature2 != b"\x00\x00\x00\x00" 
            {
                return Err(FatError::FatHeaderError("Invalid fsinfo.signature2".into()));
            }
        }
        let oem_name = String::from_utf8_lossy(&ms.ms_sysid).to_string();

        let boot_label: Option<String> = if ms.ms_ext_boot_sign == 0x29 {
            Some(String::from_utf8_lossy(&ms.ms_label).to_string())
        } else {
            None
        };

        //probe.set_fs_type(FsType::Vfat);
        //probe.set_fs_version(FsVersion::Vfat(VfatVersion::Fat32));
        //probe.set_uuid(BlkUuid::VolumeId32(VolumeId32::new(vs.vs_serno)));
        //probe.set_label_utf8_lossy(&vol_label.expect("vol_label should be valid"));
        //probe.set_usage(Usage::Filesystem);
        //probe.set_fs_block_size(vs.vs_cluster_size as u64 * sector_size as u64);
        //probe.set_block_size(sector_size as u64);
        //probe.set_fs_size(sector_size as u64 * get_sect_count(ms) as u64 );
        //probe.set_fs_extras(FsExtras::Vfat(VfatExtras { oem_name: Some(oem_name), boot_label: boot_label }));
        
        todo!();
    }

    return Err(FatError::UnknownFilesystem("Block is not fat filesystem".into()));
}
