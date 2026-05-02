#![allow(clippy::needless_return)]

#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(all(feature = "no_std", not(test)))]
extern crate core as std;

extern crate alloc;

mod error;
mod filesystem;
mod io;
mod partition;
mod probe;
mod util;

pub use crate::{
    error::Error,
    filesystem::{
        BlockFilter, BlockInfo, BlockTag, BlockType, SubType, exfat::ExFatError, ext::ExtError,
        luks::LuksError, vfat::VFatError,
    },
    io::BlockIo,
    probe::{Endianness, Id, Probe, Usage},
};

#[cfg(all(feature = "std", feature = "no_std"))]
compile_error!("`std` and `no_std` are mutually exclusive");

#[cfg(not(any(feature = "std", feature = "no_std")))]
compile_error!("must enable either `std` or `no_std`");

#[cfg(all(not(any(feature = "std", feature = "no_std")), feature = "os_calls"))]
compile_error!("`os_calls` requires `std` or `no_std`");
