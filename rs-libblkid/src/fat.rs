use thiserror::Error;
use std::str::Utf8Error;
use byteorder::{ByteOrder, LittleEndian};
use std::u16;
use std::fs::File;
use std::io::Read;
use bytemuck::checked::from_bytes;
use bytemuck::{Pod, Zeroable};
use arrayref::array_ref;

#[derive(Debug)]
pub enum FatType {
    ExFat,
    Fat32,
    Fat16,
    Fat12,
    Unknown
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct RawFatBS {
    pub bootjmp: [u8; 3],
    pub oem_name: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sector_count: u16,
    pub table_count: u8,
    pub root_entry_count: u16,
    pub total_sectors_16: u16,
    pub media_type: u8,
    pub table_size_16: u16,
    pub sectors_per_track: u16,
    pub head_side_count: u16,
    pub hidden_sector_count: u32,
    pub total_sectors_32: u32,
    pub extended_section: [u8; 476],
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct RawFatExtBs16 {
    pub bootjmp: [u8; 3],
    pub oem_name: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sector_count: u16,
    pub table_count: u8,
    pub root_entry_count: u16,
    pub total_sectors_16: u16,
    pub media_type: u8,
    pub table_size_16: u16,
    pub sectors_per_track: u16,
    pub head_side_count: u16,
    pub hidden_sector_count: u32,
    pub total_sectors_32: u32,

    pub bios_drive_num: u8,
    pub windows_nt_flags: u8,
    pub boot_signature: u8,
    pub volume_id: u32,
    pub volume_label: [u8; 11],
    pub fat_type_label: [u8; 8],
    pub executable_code: [u8; 448],
    pub boot_flag: u16,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct RawFatExtBs32 {
    pub bootjmp: [u8; 3],
    pub oem_name: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sector_count: u16,
    pub table_count: u8,
    pub root_entry_count: u16,
    pub total_sectors_16: u16,
    pub media_type: u8,
    pub table_size_16: u16,
    pub sectors_per_track: u16,
    pub head_side_count: u16,
    pub hidden_sector_count: u32,
    pub total_sectors_32: u32,
    
    pub table_size_32: u32,
    pub extended_flags: u16,
    pub fat_version: u16,
    pub root_clustor: u32,
    pub fat_info: u16,
    pub backup_bs_sector: u16,
    pub reserved_0: [u8; 12],
    pub drive_number: u8,
    pub windows_nt_flags: u8,
    pub boot_signature: u8,
    pub volume_id: u32,
    pub volume_label: [u8; 11],
    pub fat_type_label: [u8; 8],
    pub executable_code: [u8; 420],
    pub boot_flag: u16,
}

#[derive(Debug, Clone)]
pub struct Fat16Header {
    pub bootjmp: [u8; 3],
    pub oem_name: String,
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sector_count: u16,
    pub table_count: u8,
    pub root_entry_count: u16,
    pub total_sectors_16: u16,
    pub media_type: u8,
    pub table_size_16: u16,
    pub sectors_per_track: u16,
    pub head_side_count: u16,
    pub hidden_sector_count: u32,
    pub total_sectors_32: u32,
    pub extended_section: [u8; 476],
    pub boot_sector: [u8; 36],
    pub bios_drive_num: u8,
    pub windows_nt_flags: u8,
    pub boot_signature: u8,
    pub volume_id: u32,
    pub volume_label: String,
    pub fat_type_label: String,
    pub executable_code: [u8; 448],
    pub boot_flag: u16,
}

#[derive(Debug, Clone)]
pub struct Fat32Header {
    pub bootjmp: [u8; 3],
    pub oem_name: String,
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sector_count: u16,
    pub table_count: u8,
    pub root_entry_count: u16,
    pub total_sectors_16: u16,
    pub media_type: u8,
    pub table_size_16: u16,
    pub sectors_per_track: u16,
    pub head_side_count: u16,
    pub hidden_sector_count: u32,
    pub total_sectors_32: u32,
    pub table_size_32: u32,
    pub extended_flags: u16,
    pub fat_version: u16,
    pub root_clustor: u32,
    pub fat_info: u16,
    pub backup_bs_sector: u16,
    pub reserved_0: [u8; 12],
    pub drive_number: u8,
    pub windows_nt_flags: u8,
    pub boot_signature: u8,
    pub volume_id: u32,
    pub volume_label: String,
    pub fat_type_label: String,
    pub executable_code: [u8; 420],
    pub boot_flag: u16,
}

impl From<RawFatBS> for RawFatExtBs32 {
    fn from(raw: RawFatBS) -> Self {
        RawFatExtBs32 {
            bootjmp: raw.bootjmp,
            oem_name: raw.oem_name,
            bytes_per_sector: raw.bytes_per_sector,
            sectors_per_cluster: raw.sectors_per_cluster,
            reserved_sector_count: raw.reserved_sector_count,
            table_count: raw.table_count,
            root_entry_count: raw.root_entry_count,
            total_sectors_16: raw.total_sectors_16,
            media_type: raw.media_type,
            table_size_16: raw.table_size_16,
            sectors_per_track: raw.sectors_per_track,
            head_side_count: raw.head_side_count,
            hidden_sector_count: raw.hidden_sector_count,
            total_sectors_32: raw.total_sectors_32,
            table_size_32: LittleEndian::read_u32(&raw.extended_section[0..4]),
            extended_flags: LittleEndian::read_u16(&raw.extended_section[4..6]),
            fat_version: LittleEndian::read_u16(&raw.extended_section[6..8]),
            root_clustor: LittleEndian::read_u32(&raw.extended_section[8..12]),
            fat_info: LittleEndian::read_u16(&raw.extended_section[12..14]),
            backup_bs_sector: LittleEndian::read_u16(&raw.extended_section[14..16]),
            reserved_0: *array_ref![raw.extended_section, 16, 12],
            drive_number: raw.extended_section[29],
            windows_nt_flags: raw.extended_section[30],
            boot_signature: raw.extended_section[31],
            volume_id: LittleEndian::read_u32(&raw.extended_section[31..35]),
            volume_label: *array_ref![raw.extended_section, 35, 11],
            fat_type_label: *array_ref![raw.extended_section, 46, 8],
            executable_code: *array_ref![raw.extended_section, 54, 420],
            boot_flag: LittleEndian::read_u16(&raw.extended_section[474..476]),
        }
    }
}


impl From<RawFatBS> for RawFatExtBs16 {
    fn from(raw: RawFatBS) -> Self {
        RawFatExtBs16 {
            bootjmp: raw.bootjmp,
            oem_name: raw.oem_name,
            bytes_per_sector: raw.bytes_per_sector,
            sectors_per_cluster: raw.sectors_per_cluster,
            reserved_sector_count: raw.reserved_sector_count,
            table_count: raw.table_count,
            root_entry_count: raw.root_entry_count,
            total_sectors_16: raw.total_sectors_16,
            media_type: raw.media_type,
            table_size_16: raw.table_size_16,
            sectors_per_track: raw.sectors_per_track,
            head_side_count: raw.head_side_count,
            hidden_sector_count: raw.hidden_sector_count,
            total_sectors_32: raw.total_sectors_32,
            bios_drive_num: raw.extended_section[0],
            windows_nt_flags: raw.extended_section[1],
            boot_signature: raw.extended_section[2],
            volume_id: LittleEndian::read_u32(&raw.extended_section[2..6]),
            volume_label: *array_ref![raw.extended_section, 8, 11],
            fat_type_label: *array_ref![raw.extended_section, 19, 8],
            executable_code: *array_ref![raw.extended_section, 27, 448],
            boot_flag: LittleEndian::read_u16(&raw.extended_section[475..476]),
        }
    }
}


pub fn fat_type(boot_sector: RawFatBS) -> Result<FatType, Box<dyn std::error::Error>> {
    
    if boot_sector.bytes_per_sector == 0 {
        return Ok(FatType::ExFat);
    }

    let boot_sector_fat32: RawFatExtBs32 = boot_sector.into();  
    
    let total_sectors = if boot_sector.total_sectors_16 == 0 {
        boot_sector.total_sectors_32
    } else {
        boot_sector.total_sectors_16 as u32
    };
    
    let fat_size = if boot_sector.table_size_16 == 0 {
        boot_sector_fat32.table_size_32
    } else {
        boot_sector.table_size_16 as u32
    };

    let root_dir_sectors = ((boot_sector.root_entry_count as u32 * 32)
                                + (boot_sector.bytes_per_sector as u32 - 1) ) // I know this panics here if bytes_per_sector equal 0
                                / boot_sector.bytes_per_sector as u32;

    let first_data_sector = boot_sector.reserved_sector_count as u32
                                + (boot_sector.table_count as u32 * fat_size)
                                + root_dir_sectors;

    let data_sectors = total_sectors - first_data_sector;
    
    let total_clusters = data_sectors / boot_sector.sectors_per_cluster as u32;
    
    if total_clusters < 4085 {
        return Ok(FatType::Fat12);
    } else if total_clusters < 65525 {
        return Ok(FatType::Fat16);
    } else {
        return Ok(FatType::Fat32);
    }
}

pub fn read_raw_fat_bs(device: &str) -> Result<RawFatBS, Box<dyn std::error::Error>> {
    let mut raw = File::open(device)?;
    let mut buffer = [0u8; 512];
    
    raw.read_exact(&mut buffer)?;
    
    Ok(*from_bytes::<RawFatBS>(&buffer))
}

pub fn read_raw_fat16_ext_bs(device: &str) -> Result<RawFatExtBs16, Box<dyn std::error::Error>> {
    let mut raw = File::open(device)?;
    let mut buffer = [0u8; 512];
    
    raw.read_exact(&mut buffer)?;
    
    Ok(*from_bytes::<RawFatExtBs16>(&buffer))
}

pub fn read_raw_fat32_ext_bs(device: &str) -> Result<RawFatExtBs32, Box<dyn std::error::Error>> {
    let mut raw = File::open(device)?;
    let mut buffer = [0u8; 512];
    
    raw.read_exact(&mut buffer)?;
    
    Ok(*from_bytes::<RawFatExtBs32>(&buffer))
}


pub fn tests(device: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut superblock = File::open(device)?;
    let mut buffer = [0; 512];
    superblock.read_exact(&mut buffer)?;

    println!("{:X?}", buffer);
    return Ok(());
}