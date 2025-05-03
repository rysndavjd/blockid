use std::u16;
use std::fs::File;
use std::io::Read;
use byteorder::{ByteOrder, LittleEndian};
use bytemuck::checked::from_bytes;
use bytemuck::{Pod, Zeroable};
use arrayref::array_ref;


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

impl From<RawFatExtBs32> for Fat32Header {
    fn from(raw: RawFatExtBs32) -> Self {
        Fat32Header {
            bootjmp: raw.bootjmp,
            oem_name: {
                String::from_utf8_lossy(&raw.oem_name).to_string()
            },
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
            table_size_32: raw.table_size_32,
            extended_flags: raw.extended_flags,
            fat_version: raw.fat_version,
            root_clustor: raw.root_clustor,
            fat_info: raw.fat_info,
            backup_bs_sector: raw.backup_bs_sector,
            reserved_0: raw.reserved_0,
            drive_number: raw.drive_number,
            windows_nt_flags: raw.windows_nt_flags,
            boot_signature: raw.boot_signature,
            volume_id: raw.volume_id,
            volume_label: {
                String::from_utf8_lossy(&raw.volume_label).to_string()
            },
            fat_type_label: {
                String::from_utf8_lossy(&raw.fat_type_label).to_string()
            },
            executable_code: raw.executable_code,
            boot_flag: raw.boot_flag,
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

impl From<RawFatExtBs16> for RawFatBS {
    fn from(raw: RawFatExtBs16) -> Self {
        RawFatBS {
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
            extended_section: {
                let mut extended_section: [u8; 476] = [0; 476];
                extended_section[0] = raw.bios_drive_num;
                extended_section[1] = raw.windows_nt_flags;
                extended_section[2] = raw.boot_signature;
                LittleEndian::write_u32(&mut extended_section[2..6], raw.volume_id);
                extended_section[8..19].copy_from_slice(&raw.volume_label);
                extended_section[19..27].copy_from_slice(&raw.fat_type_label);
                extended_section[27..475].copy_from_slice(&raw.executable_code);
                LittleEndian::write_u16(&mut extended_section[475..476], raw.boot_flag);
                extended_section
            },
        }
    }
}

impl From<RawFatExtBs16> for Fat16Header {
    fn from(raw: RawFatExtBs16) -> Self {
        Fat16Header {
            bootjmp: raw.bootjmp,
            oem_name: {
                String::from_utf8_lossy(&raw.oem_name).to_string()
            },
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
            bios_drive_num: raw.bios_drive_num,
            windows_nt_flags: raw.windows_nt_flags,
            boot_signature: raw.boot_signature,
            volume_id: raw.volume_id,
            volume_label: {
                String::from_utf8_lossy(&raw.volume_label).to_string()
            },
            fat_type_label: {
                String::from_utf8_lossy(&raw.fat_type_label).to_string()
            },
            executable_code: raw.executable_code,
            boot_flag: raw.boot_flag,
        }
    }
}

pub fn read_raw_fat_bs(device: &str) -> Result<RawFatBS, Box<dyn std::error::Error>> {
    let mut raw = File::open(device)?;
    let mut buffer = [0u8; 512];
    
    raw.read_exact(&mut buffer)?;
    
    return Ok(*from_bytes::<RawFatBS>(&buffer));
}

pub fn read_raw_fat16_bs(device: &str) -> Result<RawFatExtBs16, Box<dyn std::error::Error>> {
    let mut raw = File::open(device)?;
    let mut buffer = [0u8; 512];
    
    raw.read_exact(&mut buffer)?;
    
    return Ok(*from_bytes::<RawFatExtBs16>(&buffer));
}

pub fn read_raw_fat32_bs(device: &str) -> Result<RawFatExtBs32, Box<dyn std::error::Error>> {
    let mut raw = File::open(device)?;
    let mut buffer = [0u8; 512];
    
    raw.read_exact(&mut buffer)?;
    
    return Ok(*from_bytes::<RawFatExtBs32>(&buffer));
}

pub fn read_fat16_bs(device: &str) -> Result<Fat16Header, Box<dyn std::error::Error>> {
    let mut raw = File::open(device)?;
    let mut buffer = [0u8; 512];
    
    raw.read_exact(&mut buffer)?;
    
    return Ok((*from_bytes::<RawFatExtBs16>(&buffer)).into());
}

pub fn read_fat32_bs(device: &str) -> Result<Fat32Header, Box<dyn std::error::Error>> {
    let mut raw = File::open(device)?;
    let mut buffer = [0u8; 512];
    
    raw.read_exact(&mut buffer)?;
    
    return Ok((*from_bytes::<RawFatExtBs32>(&buffer)).into());
}