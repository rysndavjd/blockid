pub mod luks;

use core::fmt;
use core::fmt::Debug;

use crate::BlockidError;
use crate::checksum::CsumAlgorium;

#[derive(Debug)]
pub enum ContError {
    IoError(std::io::Error),
    InvalidHeader(&'static str),
    UnknownContainer(&'static str),
    ChecksumError {
        expected: CsumAlgorium,
        got: CsumAlgorium,
    },
    NixError(rustix::io::Errno),
}

impl std::fmt::Display for ContError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContError::IoError(e) => write!(f, "I/O operation failed: {e}"),
            ContError::InvalidHeader(e) => write!(f, "Invalid Header: {e}"),
            ContError::UnknownContainer(e) => write!(f, "Unknown Container: {e}"),
            ContError::ChecksumError{expected, got} => {
                write!(f, "Container Checksum failed, expected: \"{expected:X}\" and got: \"{got:X})\"")
            },
            ContError::NixError(e) => write!(f, "*NIX error code: {e}"),
        }
    }
}

impl From<ContError> for BlockidError {
    fn from(err: ContError) -> Self {
        BlockidError::ContError(err)
    }
}