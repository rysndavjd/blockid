#![allow(clippy::needless_return)]

#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(all(feature = "no_std", not(test)))]
extern crate core as std;

extern crate alloc;

pub mod error;
pub mod filesystem;
mod io;
pub mod partition;
mod probe;
mod util;

#[cfg(feature = "os_calls")]
pub use crate::io::ioctl::AlignmentOffset;
pub use crate::{
    probe::{Endianness, Id, Probe, ProbeFlags, Usage},
    util::fd_to_path,
};

#[cfg(all(feature = "std", feature = "no_std"))]
compile_error!("`std` and `no_std` are mutually exclusive");

#[cfg(not(any(feature = "std", feature = "no_std")))]
compile_error!("must enable either `std` or `no_std`");

#[cfg(all(feature = "os_calls", not(any(feature = "std", feature = "no_std"))))]
compile_error!("`os_calls` requires `std` or `no_std`");

#[cfg(all(
    feature = "os_calls",
    not(any(target_os = "linux", target_os = "macos", target_os = "freebsd"))
))]
compile_error!("`os_calls` cannot be used on an unsupported operation system");
