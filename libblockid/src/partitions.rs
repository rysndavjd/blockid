pub mod dos;
//pub mod gpt;
//pub mod mac;
//pub mod bsd;
pub mod aix;
//pub mod solaris_x86;
//pub mod unixware;
//pub mod minix;

use crate::checksum::CsumAlgorium;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PtError {
    #[error("I/O operation failed: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid Header: {0}")]
    InvalidHeader(&'static str),
    #[error("Unknown Partition: {0}")]
    UnknownPartition(&'static str),
    #[error("Partition Checksum failed, expected: \"{expected:X}\" and got: \"{got:X})\"")]
    ChecksumError {
        expected: CsumAlgorium,
        got: CsumAlgorium,
    },
}
