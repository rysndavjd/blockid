#[cfg(feature = "std")]
pub trait SysPath: AsRef<std::path::Path> {}

#[cfg(feature = "std")]
impl<T: AsRef<std::path::Path>> SysPath for T {}

#[cfg(all(not(feature = "std"), target_family = "unix"))]
pub trait SysPath: rustix::path::Arg {}

#[cfg(all(not(feature = "std"), target_family = "unix"))]
impl<T: rustix::path::Arg> SysPath for T {}
