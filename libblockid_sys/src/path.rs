#[cfg(feature = "std")]
pub use std::path::{Path, PathBuf};

use crate::{
    alloc::borrow::Cow,
    std::{ffi::CStr, fmt::Debug},
};

pub trait Arg {
    type Error: Debug;

    fn as_str(&self) -> Result<&str, Self::Error>;
    fn to_string_lossy(&self) -> Cow<'_, str>;
}

#[cfg(feature = "std")]
impl Arg for &Path {
    type Error = std::io::Error;

    fn as_str(&self) -> Result<&str, Self::Error> {
        self.as_os_str()
            .to_str()
            .ok_or(std::io::ErrorKind::InvalidInput.into())
    }

    fn to_string_lossy(&self) -> Cow<'_, str> {
        Path::to_string_lossy(self)
    }
}

#[cfg(all(not(feature = "std"), target_family = "unix"))]
pub use crate::path::unix::Path;

#[cfg(all(not(feature = "std"), target_family = "unix"))]
mod unix {
    pub struct Path {}

    pub struct PathBuf {}
}
