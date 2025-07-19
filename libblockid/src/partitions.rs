pub mod dos;
pub mod gpt;
//pub mod mac;
//pub mod bsd;
pub mod aix;
//pub mod solaris_x86;
//pub mod unixware;
//pub mod minix;

use crate::BlockidError;
use crate::{checksum::CsumAlgorium};

#[derive(Debug)]
pub enum PtError {
    IoError(std::io::Error),
    InvalidHeader(&'static str),
    UnknownPartition(&'static str),
    ChecksumError {
        expected: CsumAlgorium,
        got: CsumAlgorium,
    }
}

impl std::fmt::Display for PtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PtError::IoError(e) => write!(f, "I/O operation failed: {e}"),
            PtError::InvalidHeader(e) => write!(f, "Invalid Header: {e}"),
            PtError::UnknownPartition(e) => write!(f, "Unknown Partition: {e}"),
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