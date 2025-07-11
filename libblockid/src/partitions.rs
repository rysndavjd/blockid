pub mod dos;
//pub mod gpt;
//pub mod mac;
pub mod bsd;
pub mod aix;
//pub mod solaris_x86;
//pub mod unixware;
//pub mod minix;

use core::fmt;

#[cfg(feature = "std")]
use std::io::Error as IoError;
#[cfg(not(feature = "std"))]
use crate::nostd_io::NoStdIoError as IoError;

use crate::BlockidError;
use crate::{checksum::CsumAlgorium};

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

#[derive(Debug)]
pub enum PtError {
    IoError(IoError),
    InvalidHeader(&'static str),
    UnknownPartition(&'static str),
    ChecksumError {
        expected: CsumAlgorium,
        got: CsumAlgorium,
    }
}

impl fmt::Display for PtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PtError::IoError(e) => write!(f, "I/O operation failed: {}", e),
            PtError::InvalidHeader(e) => write!(f, "Invalid Header: {}", e),
            PtError::UnknownPartition(e) => write!(f, "Unknown Partition: {}", e),
            PtError::ChecksumError{expected, got} => {
                write!(f, "Partition Checksum failed, expected: \"{expected:X}\" and got: \"{got:X})\"")
            },
        }
    }
}

impl From<PtError> for BlockidError {
    fn from(err: PtError) -> Self {
        BlockidError::PtError(err)
    }
}