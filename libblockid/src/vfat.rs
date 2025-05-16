use std::intrinsics::simd::simd_eq;
use std::{result, u16};
use std::fs::File;
use std::io::Read;
use byteorder::{ByteOrder, LittleEndian, BigEndian};
use bytemuck::checked::from_bytes;
use bytemuck::{Contiguous, Pod, Zeroable};
use arrayref::array_ref;

use crate::*;

const VFAT_ID_INFO: BlockIdInfo = BlockIdInfo {
    name: "vfat",
    usage: Usage::Filesystem,
    //probe: "String",
    magics: &[
        BlockMagicInfo {
            magic: b"MSWIN",
            len: 5,
            b_offset: Some(0x52),
        },
        BlockMagicInfo {
            magic: b"FAT32   ",
            len: 8,
            b_offset: Some(0x52),
        },
        BlockMagicInfo {
            magic: b"MSDOS",
            len: 5,
            b_offset: Some(0x36),
        },
        BlockMagicInfo {
            magic: b"FAT16   ",
            len: 8,
            b_offset: Some(0x36),
        },
        BlockMagicInfo {
            magic: b"FAT12   ",
            len: 8,
            b_offset: Some(0x36),
        },
        BlockMagicInfo {
            magic: b"FAT     ",
            len: 8,
            b_offset: Some(0x36),
        },
        BlockMagicInfo {
            magic: &[0xEB],
            len: 1,
            b_offset: None,
        },
        BlockMagicInfo {
            magic: &[0xE9],
            len: 1,
            b_offset: None,
        },
        BlockMagicInfo {
            magic: &[0x55, 0xAA],
            len: 2,
            b_offset: Some(0x1fe),
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

struct FatValidResult {
    cluster_count: u32,
    fat_size: u32,
    sect_count: u32,
}

const FAT12_MAX: u32 = 0xFF4;
const FAT16_MAX: u32 = 0xFFF4;
const FAT32_MAX: u32 = 0x0FFFFFF6;

const FAT_ATTR_VOLUME_ID: u32 = 0x08;
const FAT_ATTR_DIR: u32 = 0x10;
const FAT_ATTR_LONG_NAME: u32 = 0x0f;
const FAT_ATTR_MASK: u32 = 0x3f;
const FAT_ENTRY_FREE: u32 = 0xe5;

fn read_as_vfat(device: &str) -> Result<VFatSuperBlock, Box<dyn std::error::Error>> {
    let mut super_block = File::open(device)?;
    let mut buffer = [0u8; 512];
    
    super_block.read_exact(&mut buffer)?;

    return Ok(*from_bytes::<VFatSuperBlock>(&buffer));
}

fn read_as_msdos(device: &str) -> Result<MsDosSuperBlock, Box<dyn std::error::Error>> {
    let mut super_block = File::open(device)?;
    let mut buffer = [0u8; 512];
    
    super_block.read_exact(&mut buffer)?;

    return Ok(*from_bytes::<MsDosSuperBlock>(&buffer));
}

fn fat_valid_superblock(vfat: VFatSuperBlock, 
                        msdos: MsDosSuperBlock, 
                        magic: &BlockMagicInfo ) -> Result<FatValidResult, Box<dyn std::error::Error>>
{
    if magic.len <= 2 {
        if msdos.ms_pmagic[0] != 0x55 || msdos.ms_pmagic[1] != 0xAA {
            return Err("TODO ERRORS".into());
        }

        if &msdos.ms_magic == b"JFS     " || &msdos.ms_magic == b"HPFS    " {
            eprintln!("JFS/HPFS detected //Eventully proper errors/warnings will be done.");
            return Err("TODO ERRORS".into());
        }
    }

    if msdos.ms_fats >= 1  {
        return Err("TODO ERRORS".into());
    }

    if msdos.ms_media == 0xf8 || msdos.ms_media == 0xf0 {
        return Err("TODO ERRORS".into());
    }

    if !is_power_2(msdos.ms_sector_size.into()) {
        return Err("TODO ERRORS".into());
    }

    let sector_size = u16::from_le(msdos.ms_sector_size);
    if !is_power_2(sector_size.into()) || sector_size < 512 || sector_size > 4096 {
        return Err("TODO ERRORS".into());
    }

    let dir_entries: u32 = u32::from_le(msdos.ms_dir_entries.into());
    let reserved: u32 = u32::from_le(msdos.ms_reserved.into());
    let sectors: u32 = u32::from_le(msdos.ms_sectors.into());
    let clustor_size: u32 = msdos.ms_sectors.into();
    //let mut fat_length = msdos.ms_fat_length;

    let fat_length = if msdos.ms_fat_length == 0 {
        msdos.ms_fat32_length
    } else {
        msdos.ms_fat_length.into()
    };

    let fat_size: u32 = fat_length * msdos.ms_fats as u32;
    let dir_size: u32 = (dir_entries * size_of::<VfatDirEntry>() as u32) + ((sector_size-1) / sector_size) as u32;
    let cluster_count: u32 = (sectors - (reserved + fat_size + dir_size)) / clustor_size;

    let sect_count = if msdos.ms_sectors == 0 {
        msdos.ms_total_sect
    } else {
        msdos.ms_sectors.into()
    };

    let max_count = if msdos.ms_fat_length == 0 && vfat.vs_fat32_length != 0 {
        FAT32_MAX
    } else {
        if cluster_count > FAT12_MAX {
            FAT16_MAX
        } else {
            FAT12_MAX
        }
    };

    if cluster_count > max_count {
        return Err("ERROR Will make custom errors eventually".into());
    }

    return Ok(FatValidResult {
        cluster_count: cluster_count,
        fat_size: fat_size,
        sect_count: sect_count
    });
}

fn probe_vfat(device: &str,
            magic: &BlockMagicInfo) -> Result<(), Box<dyn std::error::Error>> 
{
    let vfat = read_as_vfat(device)?;
    let msdos = read_as_msdos(device)?;

    let sector_size: u32 = msdos.ms_sector_size.into();
    let reserved: u32 = msdos.ms_reserved.into();
    //let fat_size: u32 = 0;

    let version: String;
    let boot_label: [u8; 11];
    let vol_serno: u32;

    let valid = fat_valid_superblock(vfat, msdos, magic)?;

    if msdos.ms_fat_length != 0 {
        let root_start = (reserved + valid.fat_size) * sector_size;
        let root_dir_entries = vfat.vs_dir_entries;

        if msdos.ms_ext_boot_sign == 0x29 {
            boot_label = msdos.ms_label;
        }

        if msdos.ms_ext_boot_sign == 0x28 || msdos.ms_ext_boot_sign == 0x29 {
            vol_serno = msdos.ms_serno;
        }

        if valid.cluster_count < FAT12_MAX {
            version = "FAT12".to_string();
        } else if valid.cluster_count < FAT16_MAX {
            version = "FAT16".to_string();
        }
        
    } else if vfat.vs_fat32_length != 0 {
        
        /* Fat32 label extraction stuff
        let mut maxloop = 100;
        let buf_size: u32 = vfat.vs_cluster_size as u32 * sector_size;
        let start_data_sect = reserved + valid.fat_size;
        let entries = (vfat.vs_fat32_length * sector_size) / size_of::<u32>() as u32;
        let next = vfat.vs_root_cluster;

        while next != 0 && next < entries && { maxloop -= 1; maxloop != 0 } {
            let next_sect_off: u32 = (next - 2) * vfat.vs_cluster_size as u32;
            let next_off: u64 = (start_data_sect as u64 + next_sect_off as u64) * sector_size as u64;

            let count = buf_size / size_of::<VfatDirEntry>() as u32;
        }

        */
        version = "Fat32".to_string();

        //if (vs->vs_ext_boot_sign == 0x29)
		//	boot_label = vs->vs_label;
        if vfat.vs_ext_boot_sign == 0x29 {
            boot_label = vfat.vs_label;
        } else {
            boot_label = [0u8; 11];
        }

        vol_serno = vfat.vs_serno;

        
    }

    return Ok(());
}




//impl From<RawFatBS> for RawFatExtBs16 {
//    fn from(raw: RawFatBS) -> Self {
//        RawFatExtBs16 {
//            bootjmp: raw.bootjmp,
//            oem_name: raw.oem_name,
//            bytes_per_sector: raw.bytes_per_sector,
//            sectors_per_cluster: raw.sectors_per_cluster,
//            reserved_sector_count: raw.reserved_sector_count,
//            table_count: raw.table_count,
//            root_entry_count: raw.root_entry_count,
//            total_sectors_16: raw.total_sectors_16,
//            media_type: raw.media_type,
//            table_size_16: raw.table_size_16,
//            sectors_per_track: raw.sectors_per_track,
//            head_side_count: raw.head_side_count,
//            hidden_sector_count: raw.hidden_sector_count,
//            total_sectors_32: raw.total_sectors_32,
//            bios_drive_num: raw.extended_section[0],
//            windows_nt_flags: raw.extended_section[1],
//            boot_signature: raw.extended_section[2],
//            volume_id: LittleEndian::read_u32(&raw.extended_section[2..6]),
//            volume_label: *array_ref![raw.extended_section, 8, 11],
//            fat_type_label: *array_ref![raw.extended_section, 19, 8],
//            executable_code: *array_ref![raw.extended_section, 27, 448],
//            boot_flag: LittleEndian::read_u16(&raw.extended_section[475..476]),
//        }
//    }
//}


//pub fn read_raw_fat_bs(device: &str) -> Result<RawFatBS, Box<dyn std::error::Error>> {
//    let mut raw = File::open(device)?;
//    let mut buffer = [0u8; 512];
//    
//    raw.read_exact(&mut buffer)?;
//    
//    return Ok(*from_bytes::<RawFatBS>(&buffer));
//}
//
//pub fn read_raw_fat16_bs(device: &str) -> Result<RawFatExtBs16, Box<dyn std::error::Error>> {
//    let mut raw = File::open(device)?;
//    let mut buffer = [0u8; 512];
//    
//    raw.read_exact(&mut buffer)?;
//    
//    return Ok(*from_bytes::<RawFatExtBs16>(&buffer));
//}
//
//pub fn read_raw_fat32_bs(device: &str) -> Result<RawFatExtBs32, Box<dyn std::error::Error>> {
//    let mut raw = File::open(device)?;
//    let mut buffer = [0u8; 512];
//    
//    raw.read_exact(&mut buffer)?;
//    
//    return Ok(*from_bytes::<RawFatExtBs32>(&buffer));
//}
