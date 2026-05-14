#![allow(clippy::needless_return)]
#![cfg_attr(docsrs, feature(doc_cfg))]

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

#[cfg(feature = "no_std")]
#[cfg_attr(docsrs, doc(cfg(feature = "no_std")))]
pub use crate::io::path::{Path, PathBuf};
pub use crate::probe::{Endianness, Id, ProbeFlags, Usage};
#[cfg(feature = "os_calls")]
#[cfg_attr(docsrs, doc(cfg(feature = "os_calls")))]
pub use crate::{io::ioctl::AlignmentOffset, util::fd_to_path};

#[cfg(feature = "os_calls")]
#[cfg_attr(docsrs, doc(cfg(feature = "os_calls")))]
pub type Probe = RawProbe<crate::io::File>;
pub use crate::probe::RawProbe;

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
