use std::{
    fs::read_link,
    io::{Error as IoError, ErrorKind},
    path::{Path, PathBuf},
    str::Utf8Error,
};

use glob::glob;
use rustix::fs::{Dev, FileType, stat};
use thiserror::Error;
use widestring::{error::Utf16Error, utfstring::Utf16String};

use crate::{
    BlockidError, Probe, ProbeFilter, ProbeFlags,
    probe::{BlockidUUID, Endianness, ProbeResult},
};

#[derive(Debug, Error)]
pub enum UtfError {
    #[error("UTF-8 Error: {0}")]
    Utf8Error(#[from] Utf8Error),
    #[error("UTF-16 Error: {0}")]
    Utf16Error(#[from] Utf16Error),
}

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

pub fn decode_utf16_from(bytes: &[u8], endian: Endianness) -> Result<Utf16String, UtfError> {
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

    return Ok(Utf16String::from_vec(data)?);
}

pub fn decode_utf8_from(bytes: &[u8]) -> Result<String, UtfError> {
    return Ok(String::from_utf8(bytes.to_vec())
        .map_err(|e| e.utf8_error())?
        .trim_end_matches('\0')
        .to_string());
}

pub fn is_power_2(num: u64) -> bool {
    return num != 0 && ((num & (num - 1)) == 0);
}

/*
 * I think later down the line to implement a feature flag to use C functions for
 * things like this below or use the hand rolled verison to remove the need for libc
 */

/// Convert a device number (`Dev`) to a device path.
///
/// # Platform-specific
/// - Linux: uses sysfs `/sys/dev/block/<major>:<minor>`
/// - macOS/FreeBSD: uses `devname` libc function
pub fn devno_to_path(dev: Dev) -> Option<PathBuf> {
    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    {
        use libc::{S_IFBLK, c_char, dev_t, mode_t};
        use std::str::FromStr;

        unsafe extern "C" {
            unsafe fn devname(dev: dev_t, type_: mode_t) -> *const c_char;
        }

        let ptr = unsafe { devname(dev, S_IFBLK) };

        if ptr.is_null() {
            return None;
        }

        let name = unsafe { std::ffi::CStr::from_ptr(ptr) }
            .to_string_lossy()
            .to_string();

        return Some(PathBuf::from_str(&format!("/dev/{name}")).ok()?);
    }

    #[cfg(target_os = "linux")]
    {
        use rustix::fs::{major, minor};

        let path = read_link(format!("/sys/dev/block/{}:{}", major(dev), minor(dev))).ok()?;
        let target = path.file_name()?.to_str()?;

        return Some(PathBuf::from("/dev/").join(target));
    }
}

/// Convert a device path to its device number (`Dev`).
///
/// Returns [`IoError`] if:
/// - the path does not exist,
/// - or the path does not point to a block device.
///
pub fn path_to_devno<P: AsRef<Path>>(path: P) -> Result<Dev, IoError> {
    let stat = stat(path.as_ref())?;
    if FileType::from_raw_mode(stat.st_mode).is_block_device() {
        return Ok(stat.st_rdev);
    } else {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "Path doesnt point to a block device",
        ));
    }
}

/// Find the block device path corresponding to a given [`BlockidUUID`].
///
/// Iterates over common block device paths and probes each device using
/// [`Probe::from_filename`] and [`Probe::probe_values`].  
/// Returns the first device path whose UUID matches the given `uuid`.
///
/// # Errors
/// Returns [`BlockidError::NoResultPresent`] if a probe returns no result,  
/// or [`BlockidError::BlockNotFound`] if no matching device is found.
///
/// # Panics
/// Panics if glob patterns fail, which should never happen on supported systems.
///
pub fn block_from_uuid<T: Into<BlockidUUID>>(blockid_uuid: T) -> Result<PathBuf, BlockidError> {
    let uuid: BlockidUUID = blockid_uuid.into();
    log::debug!("block_from_uuid - REQUESTED UUID: {uuid:?}");

    #[cfg(target_os = "linux")]
    {
        if let Ok(buf) = read_link(format!("/dev/disk/by-uuid/{}", uuid))
            && let Some(t) = buf.file_name()
        {
            return Ok(PathBuf::from("/dev/").join(t));
        };
    }

    #[cfg(target_os = "linux")]
    let patterns = [
        "/dev/sd*",
        "/dev/hd*",
        "/dev/nvme*n*",
        "/dev/loop*",
        "/dev/ram*",
        "/dev/md*",
        "/dev/mapper/*",
    ];

    #[cfg(target_os = "freebsd")]
    let patterns = [
        "/dev/ada*",
        "/dev/da*",
        "/dev/nvd*",
        "/dev/cd*",
        "/dev/acd*",
        "/dev/md*",
    ];

    #[cfg(target_os = "macos")]
    let patterns = ["/dev/disk*"];

    for pattern in patterns {
        log::debug!("block_from_uuid - PATTERN: {pattern:?}");
        for entry in glob(pattern).expect("GLOB patterns should never fail") {
            let path = entry?;
            let stat = stat(&path)?;

            log::debug!("block_from_uuid - PATH: {path:?}");

            let mut probe =
                Probe::from_filename(&path, ProbeFlags::empty(), ProbeFilter::empty(), 0)?;

            match probe.enable_buffering_with_capacity(4096) {
                Ok(_) => (),
                Err(_) => continue,
            };

            match probe.probe_values() {
                Ok(_) => (),
                Err(_) => continue,
            };

            let value = match probe.inner_result().ok_or(BlockidError::NoResultPresent)? {
                ProbeResult::Container(r) => r.uuid,
                ProbeResult::PartTable(r) => r.uuid,
                ProbeResult::Filesystem(r) => r.uuid,
            };

            log::debug!("block_from_uuid - FOUND UUID: {value:?}");

            if FileType::from_raw_mode(stat.st_mode).is_block_device()
                && value.ok_or(BlockidError::NoResultPresent)? == uuid
            {
                return Ok(path);
            }
        }
    }
    return Err(BlockidError::BlockNotFound);
}
