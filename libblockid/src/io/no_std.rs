#[cfg(feature = "os_calls")]
pub use embedded_io::ErrorKind as IoErrorKind;
pub use embedded_io::SeekFrom;

#[cfg(feature = "os_calls")]
pub use crate::io::no_std::{
    error::IoError,
    file::File,
    path::{Path, PathBuf},
};

#[cfg(feature = "os_calls")]
mod path;

#[cfg(feature = "os_calls")]
mod file;

#[cfg(feature = "os_calls")]
mod error;
