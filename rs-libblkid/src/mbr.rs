use std::u16;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Cursor};
use byteorder::{LittleEndian, ReadBytesExt};
use uuid::Uuid;

/*
Info from https://en.wikipedia.org/wiki/Master_boot_record
*/

#[derive(Debug)]
pub struct PartitionEntry {
    pub status: u8,
    pub first_chs_address: [u8; 3],
    pub partition_type: u8, // https://en.wikipedia.org/wiki/Partition_type
    pub last_chs_address: [u8; 3],
    pub lba_first_sectors: u32,
    pub number_of_sectors: u32,
}

#[derive(Debug)]
pub struct GenericMBR {
    pub bootstrap_code_area: [u8; 446],
    pub partition_entry_1: PartitionEntry,
    pub partition_entry_2: PartitionEntry,
    pub partition_entry_3: PartitionEntry,
    pub partition_entry_4: PartitionEntry,
    pub boot_signature: [u8; 2],
}

#[derive(Debug)]
pub struct DiskTimestamp {
    pub empty_bytes: [u8; 2],
    pub physical_drive: u8,
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
}

#[derive(Debug)]
pub struct DiskSignature {
    pub signature: u32,
    pub status: u16,
}

#[derive(Debug)]
pub struct ModernMBR {
    pub bootstrap_code_area_1: [u8; 218],
    pub disk_timestamp: DiskTimestamp,
    pub bootstrap_code_area_2: [u8; 216],
    pub disk_signature: DiskSignature,
    pub partition_entry_1: PartitionEntry,
    pub partition_entry_2: PartitionEntry,
    pub partition_entry_3: PartitionEntry,
    pub partition_entry_4: PartitionEntry,
    pub boot_signature: [u8; 2],
}

pub fn check_mbr(device: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut superblock = File::open(device)?;
    // Offset cusor 510 bytes to read boot signature for MBR
    superblock.seek(SeekFrom::Start(510))?;
    let mut buffer = [0; 2];
    superblock.read_exact(&mut buffer)?;

    let mut rdr = Cursor::new(buffer);
    if rdr.read_u8()? == 0x55 && rdr.read_u8()? == 0xAA {
        return Ok(());
    } else {
        return Err("Device given is not a MBR partition table".into());
    };
}

fn read_partition_entry<R: Read>(rdr: &mut R) -> Result<PartitionEntry, Box<dyn std::error::Error>> {
    Ok(PartitionEntry { 
        status: rdr.read_u8()?, 
        first_chs_address: {
            let mut first_chs_address = [0u8; 3];
            for i in 0..3 {
                first_chs_address[i] = rdr.read_u8()?;
            }
            first_chs_address
        }, 
        partition_type: rdr.read_u8()?, 
        last_chs_address: {
            let mut last_chs_address = [0u8; 3];
            for i in 0..3 {
                last_chs_address[i] = rdr.read_u8()?;
            }
            last_chs_address
        },
        lba_first_sectors: rdr.read_u32::<LittleEndian>()?, 
        number_of_sectors: rdr.read_u32::<LittleEndian>()?,
    })
}

pub fn read_generic_mbr(device: &str) -> Result<GenericMBR, Box<dyn std::error::Error>> {
    check_mbr(device)?;

    let mut superblock = File::open(device)?;
    let mut buffer = [0; 512];
    superblock.read_exact(&mut buffer)?;
 
    let mut rdr = Cursor::new(buffer);

    return Ok(GenericMBR { 
        bootstrap_code_area: {
            let mut bootstrap_code = [0u8; 446];
            for i in 0..446 {
                bootstrap_code[i] = rdr.read_u8()?;
            }
            bootstrap_code
        },  
        partition_entry_1: read_partition_entry(&mut rdr)?,
        partition_entry_2: read_partition_entry(&mut rdr)?, 
        partition_entry_3: read_partition_entry(&mut rdr)?, 
        partition_entry_4: read_partition_entry(&mut rdr)?,
        boot_signature: {
            let mut boot_signature = [0u8; 2];
            for i in 0..2 {
                boot_signature[i] = rdr.read_u8()?;
            }
            boot_signature
        }, 
    });
}


//pub fn read_modern_mbr(device: &str) -> Result<ModernMBR, Box<dyn std::error::Error>> {
//    
//}