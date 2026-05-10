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

#[cfg(feature = "os_calls")]
pub use crate::io::ioctl::AlignmentOffset;
pub use crate::{
    error::Error,
    filesystem::{
        BlockFilter, BlockInfo, BlockTag, BlockType, SubType, apfs::ApfsError, exfat::ExFatError,
        ext::ExtError, luks::LuksError, ntfs::NtfsError, vfat::VFatError, xfs::XfsError,
    },
    io::BlockIo,
    partition::{
        PTFilter, PTType, PartAttributes, PartId, PartTableInfo, PartTableTag, PartType,
        gpt::GptError,
    },
    probe::{Endianness, Id, Probe, ProbeFlags, Usage},
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
