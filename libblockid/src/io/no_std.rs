pub use embedded_io::SeekFrom;

#[cfg(feature = "os_calls")]
pub use crate::io::no_std::{
    file::{Error, File},
    path::{Path, PathBuf},
};

#[cfg(feature = "os_calls")]
mod path;

#[cfg(feature = "os_calls")]
mod file;
