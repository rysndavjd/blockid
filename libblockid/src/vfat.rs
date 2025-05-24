use std::u16;
use std::fs::File;
use std::io::Read;
use byteorder::{ByteOrder, LittleEndian, BigEndian};
use bytemuck::checked::from_bytes;
use bytemuck::{Contiguous, Pod, Zeroable};


use crate::*;

#[derive(Debug, Clone, Copy)]
pub enum FatType {
    Fat32,
    Fat16,
    Fat12,
}

pub const VFAT_ID_INFO: BlockId = BlockId {
    name: "vfat",
    usage: Usage::Filesystem,
    //probe: "String",
    magics: &[
        BlockMagic {
            magic: b"MSWIN",
            len: 5,
            b_offset: 0x52,
        },
        BlockMagic {
            magic: b"FAT32   ",
            len: 8,
            b_offset: 0x52,
        },
        BlockMagic {
            magic: b"MSDOS",
            len: 5,
            b_offset: 0x36,
        },
        BlockMagic {
            magic: b"FAT16   ",
            len: 8,
            b_offset: 0x36,
        },
        BlockMagic {
            magic: b"FAT12   ",
            len: 8,
            b_offset: 0x36,
        },
        BlockMagic {
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
        BlockMagic {
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
    pub vs_sectors: u16, /* =0 if V3 or later */
    pub vs_media: u8,
    pub vs_fat_length: u16, /* Sectors per FAT */
    pub vs_secs_track: u16,
    pub vs_heads: u16,
    pub vs_hidden: u32,
    pub vs_total_sect: u32, /* if ms_sectors == 0 */

    pub vs_fat32_length: u32,
    pub vs_flags: u16,
    pub vs_version: u16,
    pub vs_root_cluster: u32,
    pub vs_fsinfo_sector: u16,
    pub vs_backup_boot: u16,
    pub vs_reserved2: [u8; 12],
    pub vs_drive_number: u8,
    pub vs_boot_flags: u8,
    pub vs_ext_boot_sign: u8, /* 0x28 - DOS 3.4 EBPB; 0x29 - DOS 4.0 EBPB */
    pub vs_serno: u32,
    pub vs_label: [u8; 11],
    pub vs_magic: [u8; 8],
    pub vs_dummy2: [u8; 420],
    pub vs_pmagic: [u8; 2],
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MsDosSuperBlock {
    pub ms_ignored: [u8; 3],
    pub ms_sysid: [u8; 8],
    pub ms_sector_size: u16,
    pub ms_cluster_size: u8,
    pub ms_reserved: u16,
    pub ms_fats: u8,
    pub ms_dir_entries: u16,
    pub ms_sectors: u16,
    pub ms_media: u8,
    pub ms_fat_length: u16,
    pub ms_secs_track: u16,
    pub ms_heads: u16,
    pub ms_hidden: u32,
    pub ms_total_sect: u32,

    pub ms_fat32_length: u32,
    pub ms_flags: u16,
    pub ms_version: u16,
    pub ms_root_cluster: u32,
    pub ms_fsinfo_sector: u16,
    pub ms_backup_boot: u16,
    pub ms_reserved2: [u8; 12],
    pub ms_drive_number: u8,
    pub ms_boot_flags: u8,
    pub ms_ext_boot_sign: u8,
    pub ms_serno: u32,
    pub ms_label: [u8; 11],
    pub ms_magic: [u8; 8],
    pub ms_dummy2: [u8; 420],
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

const FAT12_MAX: u32 = 0xFF4;
const FAT16_MAX: u32 = 0xFFF4;
const FAT32_MAX: u32 = 0x0FFFFFF6;

const FAT_ATTR_VOLUME_ID: u8 = 0x08;
const FAT_ATTR_DIR: u8 = 0x10;
const FAT_ATTR_LONG_NAME: u8 = 0x0f;
const FAT_ATTR_MASK: u8 = 0x3f;
const FAT_ENTRY_FREE: u8 = 0xe5;

pub fn read_as_vfat(
        raw_block: &File
    ) -> Result<VFatSuperBlock, Box<dyn std::error::Error>> 
{
    let mut block = raw_block.try_clone()?;
    block.seek(SeekFrom::Start(0))?;

    let mut buffer = [0u8; 512];
    block.read_exact(&mut buffer)?;

    return Ok(*from_bytes::<VFatSuperBlock>(&buffer));
}

pub fn read_as_msdos(
        raw_block: &File
    ) -> Result<MsDosSuperBlock, Box<dyn std::error::Error>> 
{
    let mut block = raw_block.try_clone()?;
    block.seek(SeekFrom::Start(0))?;

    let mut buffer = [0u8; 512];
    block.read_exact(&mut buffer)?;

    return Ok(*from_bytes::<MsDosSuperBlock>(&buffer));
}

fn read_vfat_dir_entry(
        raw_block: &File,
        offset: u32,
    ) -> Result<VfatDirEntry, Box<dyn std::error::Error>> 
{
    let mut block = raw_block.try_clone()?;
    block.seek(SeekFrom::Start(0))?;

    let mut buffer = [0u8; 32];
    block.seek(SeekFrom::Start(offset.into()))?;
    block.read_exact(&mut buffer)?;

    return Ok(*from_bytes::<VfatDirEntry>(&buffer));
}

struct ValidFatResult {
    fat_size: u32,
    cluster_count: u32,
    sect_count: u32,
}

fn valid_fat (
        ms: MsDosSuperBlock,
        vs: VFatSuperBlock,
        mag: BlockMagic,
    ) -> Result<ValidFatResult ,Box<dyn std::error::Error>> 
{    
    if mag.len <= 2 {
        if ms.ms_pmagic[0] != 0x55 || ms.ms_pmagic[1] != 0xAA {
            return Err("Given block is not Fat likely MBR".into());
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
            return Err("JFS/HPFS found".into());
        }
    }

    if ms.ms_fats == 0 {
        return Err("Should be atleast one fat table".into());
    }
    if ms.ms_reserved == 0 {
        return Err("ms_reserved should not be 0".into());
    }

    if !is_power_2(ms.ms_cluster_size.into()) {
        return Err("cluster_size is not ^2".into());
    }

    let sector_size: u32 = ms.ms_sector_size.into();
    let dir_entries: u32 = ms.ms_dir_entries.into();
    let reserved: u32 = ms.ms_reserved.into();
    let num_fat: u32 = ms.ms_fats.into();
    let cluster_size: u32 = ms.ms_cluster_size.into();

    let sect_count: u32 = if ms.ms_sectors == 0 {
        ms.ms_total_sect
    } else {
        ms.ms_sectors.into()
    };
    
    let fat_length: u32 = if ms.ms_fat_length == 0 {
        vs.vs_fat32_length
    } else {
        ms.ms_fat_length.into()
    };

    let fat_size: u32 = fat_length * num_fat;
    let dir_size: u32 = (dir_entries * 32) + (sector_size - 1) / sector_size;

    let cluster_count: u32 = (sect_count - (reserved + fat_size + dir_size)) / cluster_size;

    let max_count = if ms.ms_fat_length == 0 && vs.vs_fat32_length > 0 {
        println!("Fat32");
        FAT32_MAX
    } else if cluster_count > FAT12_MAX {
        println!("Fat16");
        FAT16_MAX
    } else {
        println!("Fat12");
        FAT12_MAX
    };

    if cluster_count > max_count {
        return Err("Too many clusters".into());
    }
    
    if cluster_count < FAT12_MAX || cluster_count < FAT16_MAX || cluster_count < FAT32_MAX {
        return Ok(ValidFatResult {
            fat_size: fat_size, 
            cluster_count: cluster_count,
            sect_count: sect_count, 
        });
    } else {
        return Err("Unknown fat type".into());
    }

}

pub fn probe_is_vfat(
        probe: &mut BlockProbe,
    ) -> Result<(), Box<dyn std::error::Error>>
{
    let ms: MsDosSuperBlock = read_as_msdos(&probe.file)?;
    let vs: VFatSuperBlock = read_as_vfat(&probe.file)?;

    let mag: BlockMagic = probe_get_magic(probe, &VFAT_ID_INFO)?;
    
    valid_fat(ms, vs, mag)?;

    return Ok(());
}

fn search_fat_label(
        probe: &mut BlockProbe,
        root_start: u32,
        root_dir_entries: u32,
    ) -> Result<[u8; 11], Box<dyn std::error::Error>> 
{
    let vfat_dir_entry: Option<VfatDirEntry> = if !probe.is_tiny() {
        Some(read_vfat_dir_entry(&probe.file, root_start)?)
    } else {
        None
    };

    for i in 0..root_dir_entries {
        let entry = if vfat_dir_entry.is_none() {
            read_vfat_dir_entry(&probe.file, root_start + (i * 32))?
        } else {
            vfat_dir_entry.expect("Should have a value")
        };

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
            return Ok(label);
        }
    }

    return Err("Unable to get fat label".into());
}

fn probe_vfat(
        probe: &mut BlockProbe,
        mag: BlockMagic,
    ) -> Result<() ,Box<dyn std::error::Error>> 
{
    let ms = read_as_msdos(&probe.file)?;
    let vs = read_as_vfat(&probe.file)?;

    let valid_info = valid_fat(ms, vs, mag)?;

    let sector_size: u32 = ms.ms_sector_size.into();
    let reserved: u32 = ms.ms_reserved.into();

    let vol_label: [u8; 11];
    let boot_label: [u8; 11];
    let vol_serno: FsUuid;
    let version: String;

    if ms.ms_fat_length != 0 {
        let root_start: u32 = (reserved + valid_info.fat_size) * sector_size;
        let root_dir_entries: u32 = vs.vs_dir_entries.into();

        vol_label = search_fat_label(probe, root_start, root_dir_entries)?;

        if ms.ms_ext_boot_sign == 0x29 {
            boot_label = ms.ms_label;
        }

        if ms.ms_ext_boot_sign == 0x28 || ms.ms_ext_boot_sign == 0x29 {
            vol_serno = FsUuid::VolumeId32(ms.ms_serno);
        }

        if valid_info.cluster_count < FAT12_MAX {
            version = "FAT12".to_string();
        } else if valid_info.cluster_count < FAT16_MAX {
            version = "FAT16".to_string();
        }

    } else if vs.vs_fat32_length != 0 {
        let mut buffer: [u8; 11];
        
    }

    return Ok(());
}
