#[cfg(all(feature = "os_calls", target_os = "macos"))]
mod macos;

use widestring::U16String;

use crate::Endianness;
#[cfg(feature = "os_calls")]
use crate::{
    error::PartToDiskError,
    io::{IoError, Path, PathBuf},
};

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
pub fn fd_to_path<F: rustix::fd::AsRawFd>(fd: F) -> Result<PathBuf, IoError> {
    #[cfg(target_os = "linux")]
    {
        let link = rustix::fs::readlink(format!("/proc/self/fd/{}", fd.as_raw_fd()), Vec::new())?;

        #[cfg(feature = "std")]
        {
            use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

            return Ok(PathBuf::from(OsStr::from_bytes(link.as_bytes())));
        }

        #[cfg(feature = "no_std")]
        return Ok(PathBuf::from(link.as_bytes()));
    }

    #[cfg(target_os = "macos")]
    {
        use libc::{__error, F_GETPATH, PATH_MAX, fcntl};
        use rustix::io::Errno;

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

#[cfg(feature = "os_calls")]
pub fn part_to_disk<P: AsRef<Path>>(path: P) -> Result<PathBuf, PartToDiskError<IoError>> {
    #[cfg(target_os = "linux")]
    {
        use rustix::fs::{Access, access, major, minor, readlink, stat};

        #[cfg(feature = "std")]
        let path = path.as_ref();
        #[cfg(feature = "no_std")]
        let path = path.as_ref().as_bytes();

        let dev = stat(path)?.st_rdev;

        let sys_path = format!("/sys/dev/block/{}:{}", major(dev), minor(dev));

        if access(format!("{sys_path}/partition"), Access::EXISTS).is_ok() {
            let link = readlink(sys_path, Vec::new())?;

            let last = link
                .as_bytes()
                .iter()
                .rposition(|b| b == &b'/')
                .ok_or(PartToDiskError::DiskNotPartition)?;

            let second_last = link.as_bytes()[..last]
                .iter()
                .rposition(|b| b == &b'/')
                .ok_or(PartToDiskError::DiskNotPartition)?;

            let disk = &link.as_bytes()[second_last.saturating_add(1)..last];

            #[cfg(feature = "std")]
            {
                use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

                return Ok(Path::new("/dev").join(OsStr::from_bytes(disk)));
            }
            #[cfg(feature = "no_std")]
            {
                return Ok(Path::new("/dev").join(disk));
            }
        }
        return Err(PartToDiskError::DiskNotPartition);
    }

    #[cfg(target_os = "macos")]
    {
        // I separated the macOS code into a separate module due to the ffi
        // interfaces and other junk that would just clutter this file up.
        macos::part_to_disk(path)
    }

    #[cfg(target_os = "freebsd")]
    {
        todo!()
    }
}
