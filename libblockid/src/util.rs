use widestring::{error::Utf16Error, utfstring::Utf16String};

use crate::{probe::Endianness, std::str::Utf8Error};

pub fn decode_utf16_lossy_from(bytes: &[u8], endian: Endianness) -> Utf16String {
    let data: Vec<u16> = bytes
        .chunks(2)
        .filter_map(|chunk| {
            if chunk.len() == 2 {
                let val = match endian {
                    Endianness::Big => u16::from_be_bytes([chunk[0], chunk[1]]),
                    Endianness::Little => u16::from_le_bytes([chunk[0], chunk[1]]),
                };
                if val == 0 { None } else { Some(val) }
            } else {
                None
            }
        })
        .collect();

    return Utf16String::from_slice_lossy(&data).into();
}

pub fn decode_utf8_lossy_from(bytes: &[u8]) -> String {
    return String::from_utf8_lossy(bytes)
        .trim_end_matches('\0')
        .to_string();
}

pub fn decode_utf16_from(bytes: &[u8], endian: Endianness) -> Result<Utf16String, Utf16Error> {
    let data: Vec<u16> = bytes
        .chunks(2)
        .filter_map(|chunk| {
            if chunk.len() == 2 {
                let val = match endian {
                    Endianness::Big => u16::from_be_bytes([chunk[0], chunk[1]]),
                    Endianness::Little => u16::from_le_bytes([chunk[0], chunk[1]]),
                };
                if val == 0 { None } else { Some(val) }
            } else {
                None
            }
        })
        .collect();

    return Utf16String::from_vec(data);
}

pub fn decode_utf8_from(bytes: &[u8]) -> Result<String, Utf8Error> {
    return Ok(String::from_utf8(bytes.to_vec())
        .map_err(|e| e.utf8_error())?
        .trim_end_matches('\0')
        .to_string());
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
/// # `std`
/// The path bytes are decoded into a UTF-8 [`String`] lossily before assembling the
/// standard library [`PathBuf`](https://doc.rust-lang.org/std/path/struct.PathBuf.html).
///
/// # `no_std`
/// The raw bytes are used as is to assemble the crate wrapper type `PathBuf`.
///
/// [`fcntl`]: https://docs.rs/libc/latest/libc/fn.fcntl.html
#[cfg(feature = "os_calls")]
pub fn fd_to_path<F: rustix::fd::AsRawFd>(
    fd: F,
) -> Result<crate::io::PathBuf, crate::error::Error<crate::io::IoError>> {
    #[cfg(target_os = "linux")]
    {
        use rustix::fs::readlink;

        use crate::io::PathBuf;

        let link = readlink(format!("/proc/self/fd/{}", fd.as_raw_fd()), Vec::new())?;

        #[cfg(feature = "std")]
        return Ok(PathBuf::from(link.to_string_lossy().to_string()));
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
        return Ok(PathBuf::from(decode_utf8_lossy_from(&buf)));
        #[cfg(feature = "no_std")]
        return Ok(PathBuf::from(buf.as_slice()));
    }

    #[cfg(target_os = "freebsd")]
    {
        todo!()
    }
}
