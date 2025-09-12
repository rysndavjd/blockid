pub mod dos;
//pub mod gpt;
//pub mod mac;
//pub mod bsd;
pub mod aix;
//pub mod solaris_x86;
//pub mod unixware;
//pub mod minix;

use thiserror::Error;

use crate::partitions::dos::DosPTError;

#[derive(Debug, Error)]
pub enum PtError {
    #[error("DOS/MBR partition table error: {0}")]
    Dos(#[from] DosPTError),
}
