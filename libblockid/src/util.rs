use widestring::U16String;

use crate::Endianness;

pub(crate) fn bytes_to_u16string(bytes: &[u8], endianness: Endianness) -> U16String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| match endianness {
            Endianness::Little => u16::from_le_bytes([c[0], c[1]]),
            Endianness::Big => u16::from_be_bytes([c[0], c[1]]),
        })
        .collect();

    U16String::from_vec(units)
}

/// Gets the path of a file descriptor, returning a [`PathBuf`](crate::io::PathBuf).
///
/// # Platform-specific behavior
///
/// ## Linux
/// Uses the `/proc` filesystem via `/proc/self/fd/<fd>`, reading the symlink target.
///
/// ## macOS
/// Uses [`fcntl`] with `F_GETPATH` to retrieve the path.
///
/// ## FreeBSD
/// TODO!.
///
/// [`fcntl`]: https://docs.rs/libc/latest/libc/fn.fcntl.html
#[cfg(feature = "os_calls")]
pub fn fd_to_path<F: rustix::fd::AsRawFd>(
    fd: F,
) -> Result<crate::io::PathBuf, crate::error::Error<crate::io::IoError>> {
    #[cfg(target_os = "linux")]
    {
        use crate::io::PathBuf;

        let link = rustix::fs::readlink(format!("/proc/self/fd/{}", fd.as_raw_fd()), Vec::new())?;

        #[cfg(feature = "std")]
        {
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;

            return Ok(PathBuf::from(OsStr::from_bytes(link.as_bytes())));
        }

        #[cfg(feature = "no_std")]
        return Ok(PathBuf::from(link.as_bytes()));
    }

    #[cfg(target_os = "macos")]
    {
        use libc::{__error, F_GETPATH, PATH_MAX, fcntl};
        use rustix::io::Errno;

        use crate::io::PathBuf;

        let mut buf = [0u8; PATH_MAX as usize];
        let ret = unsafe { fcntl(fd.as_raw_fd(), F_GETPATH, buf.as_mut_ptr()) };

        if ret == -1 {
            return Err(Errno::from_raw_os_error(unsafe { *__error() }).into());
        }

        #[cfg(feature = "std")]
        {
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;

            return Ok(PathBuf::from(OsStr::from_bytes(&buf)));
        }

        #[cfg(feature = "no_std")]
        return Ok(PathBuf::from(buf.as_slice()));
    }

    #[cfg(target_os = "freebsd")]
    {
        todo!()
    }
}
