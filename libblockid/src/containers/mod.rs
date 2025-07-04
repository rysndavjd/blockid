pub mod luks;

use std::io;
use thiserror::Error;
use crate::checksum::CsumAlgorium;

#[derive(Error, Debug)]
pub enum ContError {
    #[error("I/O operation failed: {0}")]
    IoError(#[from] io::Error),
    #[error("Invalid Header: {0}")]
    InvalidHeader(&'static str),
    #[error("Unknown Container: {0}")]
    UnknownContainer(&'static str),
    #[error("Container Checksum failed, expected: \"{expected:X}\" and got: \"{got:X})\"")]
    ChecksumError {
        expected: CsumAlgorium,
        got: CsumAlgorium,
    },
    #[error("*Nix operation failed: {0}")]
    NixError(#[from] rustix::io::Errno),
}