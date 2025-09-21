pub mod luks;
pub mod lvm;

use thiserror::Error;

use crate::containers::{luks::LuksError, lvm::LvmError};

#[derive(Debug, Error)]
pub enum ContError {
    #[error("LUKS container error: {0}")]
    LuksError(#[from] LuksError),
    #[error("LUKS container error: {0}")]
    LvmError(#[from] LvmError),
}
