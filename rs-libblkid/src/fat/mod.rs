pub mod fat32_16;
pub mod exfat;

use fat32_16::*;
use std::fs::File;
use std::io::Read;

#[derive(Debug, PartialEq)]
pub enum FatType {
    ExFat,
    Fat32,
    Fat16,
    Fat12,
}

pub fn fat_type(device: &str) -> Result<FatType, Box<dyn std::error::Error>> {
    let boot_sector = read_raw_fat_bs(device)?;

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

pub fn check_fat16(device: &str) -> Result<(), Box<dyn std::error::Error>> {
    if read_raw_fat16_bs(device)?.boot_flag == 0xAA55 && fat_type(device)? == FatType::Fat16 {
        return Ok(());
    } else {
        return Err("That is not Fat16".into());
    }
}

pub fn check_fat32(device: &str) -> Result<(), Box<dyn std::error::Error>> {
    if read_raw_fat32_bs(device)?.boot_flag == 0xAA55 && fat_type(device)? == FatType::Fat32 {
        return Ok(());
    } else {
        return Err("That is not Fat32".into());
    }
}

//pub fn check_exfat(device: &str) -> Result<(), Box<dyn std::error::Error>> {
//    
//}


pub fn testss(device: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut superblock = File::open(device)?;
    let mut buffer = [0; 512];
    superblock.read_exact(&mut buffer)?;

    println!("{:X?}", buffer);
    return Ok(());

}