pub mod luks;

use thiserror::Error;

use crate::containers::luks::LuksError;

#[derive(Debug, Error)]
pub enum ContError {
    #[error("LUKS container error: {0}")]
    LuksError(#[from] LuksError),
}
