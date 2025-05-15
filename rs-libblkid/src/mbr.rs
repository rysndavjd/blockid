use std::u16;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Cursor};
use byteorder::{LittleEndian, ReadBytesExt};

/*
Info from https://en.wikipedia.org/wiki/Master_boot_record
*/

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct PartitionEntry {
    pub status: u8,
    pub first_chs_address: [u8; 3],
    pub partition_type: u8, // https://en.wikipedia.org/wiki/Partition_type
    pub last_chs_address: [u8; 3],
    pub lba_first_sectors: u32,
    pub number_of_sectors: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct GenericMBR {
    pub bootstrap_code_area: [u8; 446],
    pub partition_entry_1: PartitionEntry,
    pub partition_entry_2: PartitionEntry,
    pub partition_entry_3: PartitionEntry,
    pub partition_entry_4: PartitionEntry,
    pub boot_signature: [u8; 2],
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DiskTimestamp {
    pub empty_bytes: [u8; 2],
    pub physical_drive: u8,
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DiskSignature {
    pub signature: u32,
    pub status: u16,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
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
