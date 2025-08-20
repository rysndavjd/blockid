pub mod exfat;
pub mod ext;
pub mod linux_swap;
pub mod ntfs;
pub mod vfat;
pub mod xfs;
pub mod volume_id;

use thiserror::Error;
use crate::{checksum::CsumAlgorium, util::UtfError};

#[derive(Debug, Error)]
pub enum FsError {
    #[error("I/O operation failed: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid Header: {0}")]    
    InvalidHeader(&'static str),
    #[error("Unknown Filesystem: {0}")]
    UnknownFilesystem(&'static str),
    #[error("UTF Error: {0}")]
    UtfError(#[from] UtfError),
    #[error("Filesystem Checksum failed, expected: \"{expected:X}\" and got: \"{got:X})\"")]
    ChecksumError {
        expected: CsumAlgorium,
        got: CsumAlgorium,
    }
}
