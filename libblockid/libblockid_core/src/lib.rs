#![allow(clippy::needless_return)]

#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(all(not(feature = "std"), not(test)))]
extern crate core as std;

mod error;
mod filesystem;
mod probe;
mod util;

#[cfg(feature = "std")]
use std::io::{ErrorKind as IoErrorKind, Read, Seek, SeekFrom};

#[cfg(not(feature = "std"))]
use embedded_io::{ErrorKind as IoErrorKind, Read, Seek, SeekFrom};

pub use crate::error::Error;
