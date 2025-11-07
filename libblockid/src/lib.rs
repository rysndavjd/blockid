//! `libblockid` provides tools for probing block devices, partitions,
//! filesystems, and containers. It allows detection of device types,
//! UUIDs, labels, and other metadata, exposing structured results via
//! [`Probe`] and related result types.

#![allow(clippy::needless_return)]
#![warn(missing_docs)]

mod probe;

#[cfg(test)]
mod tests;

pub(crate) mod ioctl;
mod util;

pub(crate) mod containers;
pub(crate) mod filesystems;
pub(crate) mod partitions;

use std::{
    fs::File,
    io::{Error as IoError, ErrorKind as IoErrorKind},
    path::{Path, PathBuf},
};

use glob::GlobError;
use rustix::fs::Dev;
use thiserror::Error;

use crate::{containers::ContError, filesystems::FsError, partitions::PtError};

pub use crate::{
    filesystems::volume_id::{VolumeId32, VolumeId64},
    probe::{
        BlockidMagic, BlockidUUID, PROBES, Probe, ProbeFilter, ProbeFlags, 
        ProbeResult,
    },
    util::{block_from_uuid, devno_to_path, path_to_devno},
};

/// Represents all possible errors that can occur during probing and block inspection.
///
/// This enum consolidates errors from globbing, I/O, filesystem, partition table,
/// container probing, and low-level OS interactions.
#[derive(Debug, Error)]
pub enum BlockidError {
    /// Error occurred while expanding a glob pattern.
    #[error("Glob Error: {0}")]
    GlobError(#[from] GlobError),
    /// Invalid argument(s) were provided to a function.
    #[error("Invalid Arguments given: {0}")]
    ArgumentError(&'static str),
    /// A generic result error with a static message.
    #[error("Result Error: {0}")]
    ResultError(&'static str),
    /// All implemented probes were exhausted without detecting a known type.
    #[error("All implemented probes exhausted")]
    ProbesExhausted,
    /// Probe completed, but no result is present.
    #[error("No Result is present")]
    NoResultPresent,
    /// Block corresponding to a UUID or path was not found.
    #[error("Block was not found")]
    BlockNotFound,
    /// A filesystem probe failed.
    #[error("Filesystem probe failed: {0}")]
    FsError(#[from] FsError),
    /// A partition table probe failed.
    #[error("Partition Table probe failed: {0}")]
    PtError(#[from] PtError),
    /// A container (e.g., LUKS) probe failed.
    #[error("Container probe failed: {0}")]
    ContError(#[from] ContError),
    /// An I/O operation failed.
    #[error("I/O operation failed: {0}")]
    IoError(#[from] IoError),
    /// A low-level *nix operation failed.
    #[error("*Nix operation failed: {0}")]
    NixError(#[from] rustix::io::Errno),
}

#[derive(Debug, Clone)]
enum IdType {
    Path(PathBuf),
    Devno(Dev),
}

/// Builder pattern for creating a [`Probe`] with configurable options.
///
/// Allows setting the device by path or dev number, specifying an offset,
/// probe flags, and filters.
#[derive(Debug, Default, Clone)]
pub struct ProbeBuilder {
    disk_id: Option<IdType>,
    offset: u64,
    flags: ProbeFlags,
    filter: ProbeFilter,
}

impl ProbeBuilder {
    /// Creates a new [`ProbeBuilder`] with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the device to probe using a filesystem path.
    pub fn path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.disk_id = Some(IdType::Path(path.as_ref().to_path_buf()));
        self
    }

    /// Sets the device to probe using a device number [`Dev`].
    pub fn devno(mut self, devno: Dev) -> Self {
        self.disk_id = Some(IdType::Devno(devno));
        self
    }

    /// Sets the byte offset from which to start probing.
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = offset;
        self
    }

    /// Sets probe flags [`ProbeFlags`] to customize behavior.
    pub fn flags(mut self, flags: ProbeFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Sets probe filters [`ProbeFilter`] to skip certain checks.
    pub fn filter(mut self, filter: ProbeFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Builds a [`Probe`] instance from the current builder configuration.
    ///
    /// # Errors
    /// Returns [`BlockidError::ArgumentError`] if no path or dev number was set.
    /// Returns [`IoError`] if opening the device or path fails.
    pub fn build(self) -> Result<Probe, BlockidError> {
        let id = self.disk_id.ok_or(BlockidError::ArgumentError(
            "Path/devno not set in ProbeBuilder",
        ))?;

        let (file, path) = match id {
            IdType::Path(path) => (File::open(&path)?, path),
            IdType::Devno(devno) => {
                let path = devno_to_path(devno).ok_or(IoError::new(
                    IoErrorKind::InvalidInput,
                    "Devno doesnt point to a path",
                ))?;
                (File::open(&path)?, path)
            }
        };
        Probe::new(file, &path, self.offset, self.flags, self.filter)
    }
}
