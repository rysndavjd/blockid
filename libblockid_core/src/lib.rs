#![allow(clippy::needless_return)]

#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(all(not(feature = "std"), not(test)))]
extern crate core as std;

extern crate alloc;

mod error;
mod filesystem;
mod io;
mod probe;
mod util;

pub use crate::{
    error::Error,
    io::BlockIo,
    probe::{BlockInfo, BlockTag, BlockType, Endianness, Filter, Id, LowProbe, SecType, Usage},
};

#[cfg(all(feature = "std", feature = "no_std"))]
compile_error!("features `std` and `no_std` are mutually exclusive");

#[cfg(not(any(feature = "std", feature = "no_std")))]
compile_error!("must enable either `std` or `no_std` feature");
