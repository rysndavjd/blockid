pub mod exfat;
pub mod ext;
pub mod linux_swap;
pub mod ntfs;
pub mod vfat;
pub mod volume_id;

use crate::util::UtfError;
use crate::BlockidError;
use crate::checksum::CsumAlgorium;

#[derive(Debug)]
pub enum FsError {
    IoError(std::io::Error),
    InvalidHeader(&'static str),
    UnknownFilesystem(&'static str),
    UtfError(UtfError),
    ChecksumError {
        expected: CsumAlgorium,
        got: CsumAlgorium,
    }
}

impl std::fmt::Display for FsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsError::IoError(e) => write!(f, "I/O operation failed: {e}"),
            FsError::InvalidHeader(e) => write!(f, "Invalid Header: {e}"),
            FsError::UnknownFilesystem(e) => write!(f, "Unknown Filesystem: {e}"),
            FsError::UtfError(e) => write!(f, "UTF Error: {e}"),
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
