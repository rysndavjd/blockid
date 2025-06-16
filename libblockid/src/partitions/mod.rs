//pub mod dos;
//pub mod gpt;
//pub mod mac;
//pub mod bsd;
//pub mod aix;
//pub mod solaris_x86;
//pub mod unixware;
//pub mod minix;

use crate::{checksum::CsumAlgorium, BlockidUUID};
use rustix::fs::Dev;
use uuid::Uuid;
use thiserror::Error;
use std::io;

/*
  PTTYPE:               partition table type (dos, gpt, etc.).
  PTUUID:               partition table id (uuid for gpt, hex for dos).
  PART_ENTRY_SCHEME:    partition table type
  PART_ENTRY_NAME:      partition name (gpt and mac only)
  PART_ENTRY_UUID:      partition UUID (gpt, or pseudo IDs for MBR)
  PART_ENTRY_TYPE:      partition type, 0xNN (e.g. 0x82) or type UUID (gpt only) or type string (mac)
  PART_ENTRY_FLAGS:     partition flags (e.g. boot_ind) or  attributes (e.g. gpt attributes)
  PART_ENTRY_NUMBER:    partition number
  PART_ENTRY_OFFSET:    the begin of the partition
  PART_ENTRY_SIZE:      size of the partition
  PART_ENTRY_DISK:      whole-disk maj:min
*/


#[derive(Error, Debug)]
pub enum PtError {
    #[error("I/O operation failed")]
    IoError(#[from] io::Error),
    #[error("Invalid Header: {0}")]
    InvalidHeader(String),
    #[error("Unknown Partition: {0}")]
    UnknownPartition(String),
    #[error("Checksum failed, expected: \"{expected:?}\" and got: \"{got:?})\"")]
    ChecksumError {
        expected: CsumAlgorium,
        got: CsumAlgorium,
    }
}

#[derive(Debug)]
pub enum PartitionItem {
    PartTable(BlockidPartTable),
    Partition(BlockidPartition)
}

#[derive(Debug)]
pub struct BlockidPartTable {
    pub start: u64,
    pub size: u64,

    pub pt_type: PTType,
    pub flags: PTflags,
    pub table_uuid: BlockidUUID,

    pub partitions: Vec<PartitionItem>,
}

#[derive(Debug)]
pub struct BlockidPartition {
    start: u64,
    size: u64,

    dev: Dev,
    partno: u16,

    name: String,
    uuid: BlockidUUID,
    entry_type: PTType,
}

#[derive(Debug, Clone)]
pub enum PTType {
    #[cfg(feature = "dos")]
    Dos,
    #[cfg(feature = "gpt")]
    Gpt,
    #[cfg(feature = "mac")]
    Mac,
    #[cfg(feature = "bsd")]
    Bsd,
    Unknown(String), 
}

#[derive(Debug, Clone)]
pub enum PartEntryType {
    Byte(u8),
    Uuid(Uuid),
}

#[derive(Debug, Clone)]
pub enum PTflags {
    Unknown(String), 
}
