pub mod exfat;
pub mod ext;
pub mod linux_swap;
pub mod ntfs;
pub mod vfat;
pub mod volume_id;

use core::fmt;
use core::fmt::Debug;

#[cfg(feature = "std")]
use std::io::Error as IoError;
#[cfg(not(feature = "std"))]
use crate::nostd_io::NoStdIoError as IoError;

use crate::BlockidError;
use crate::checksum::CsumAlgorium;

/* Tags
TYPE:           filesystem type
SEC_TYPE:       Secondary filesystem type
LABEL:          fs label
LABEL_RAW:      Raw fs label
UUID:           fs uuid
UUID_RAW:       raw uuid
UUID_SUB:       Sub uuid
LOG_UUID:       external log uuid
LOG_UUID_RAW:   external log uuid
EXT_JOURNAL:    external journal uuid
USAGE:          usage string 
VERSION:        fs version
SBMAGIC:        superblock magic string
SBMAGIC_OFFSET: magic offset
FSSIZE:         size of filesystem
FSLASTBLOCK:    offset of last sector in superblock   
FSBLOCKSIZE:    fs block size
BLOCK_SIZE:     block size of phyical disk
*/

#[derive(Debug)]
pub enum FsError {
    IoError(IoError),
    InvalidHeader(&'static str),
    UnknownFilesystem(&'static str),
    ChecksumError {
        expected: CsumAlgorium,
        got: CsumAlgorium,
    }
}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FsError::IoError(e) => write!(f, "I/O operation failed: {}", e),
            FsError::InvalidHeader(e) => write!(f, "Invalid Header: {}", e),
            FsError::UnknownFilesystem(e) => write!(f, "Unknown Filesystem: {}", e),
            FsError::ChecksumError{expected, got} => {
                write!(f, "Filesystem Checksum failed, expected: \"{expected:X}\" and got: \"{got:X})\"")
            },
        }
    }
}

impl From<FsError> for BlockidError {
    fn from(err: FsError) -> Self {
        BlockidError::FsError(err)
    }
}

pub fn is_power_2(num: u64) -> bool {
    return num != 0 && ((num & (num - 1)) == 0); 
}