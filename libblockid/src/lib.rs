#![allow(clippy::needless_return)]

mod probe;

#[cfg(test)]
mod tests;

pub(crate) mod checksum;
pub(crate) mod ioctl;
mod util;

pub(crate) mod containers;
pub(crate) mod filesystems;
pub(crate) mod partitions;

use std::{
    fs::File,
    io::Error as IoError,
    path::{Path, PathBuf},
};

use glob::GlobError;
use rustix::fs::Dev;
use thiserror::Error;

use crate::{containers::ContError, filesystems::FsError, partitions::PtError};

pub use crate::{
    probe::{Probe, ProbeFilter, ProbeFlags},
    util::{devno_to_path, path_to_devno},
};

#[derive(Debug, Error)]
pub enum BlockidError {
    #[error("Glob Error: {0}")]
    GlobError(#[from] GlobError),
    #[error("Invalid Arguments given: {0}")]
    ArgumentError(&'static str),
    #[error("Result Error: {0}")]
    ResultError(&'static str),
    #[error("Probe failed: {0}")]
    ProbeError(&'static str),
    #[error("Filesystem probe failed: {0}")]
    FsError(#[from] FsError),
    #[error("Partition Table probe failed: {0}")]
    PtError(#[from] PtError),
    #[error("Container probe failed: {0}")]
    ContError(#[from] ContError),
    #[error("I/O operation failed: {0}")]
    IoError(#[from] IoError),
    #[error("*Nix operation failed: {0}")]
    NixError(#[from] rustix::io::Errno),
}

#[derive(Debug, Clone)]
enum IdType {
    Path(PathBuf),
    Devno(Dev),
}

#[derive(Debug, Default, Clone)]
pub struct ProbeBuilder {
    disk_id: Option<IdType>,
    offset: u64,
    flags: ProbeFlags,
    filter: ProbeFilter,
}

impl ProbeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.disk_id = Some(IdType::Path(path.as_ref().to_path_buf()));
        self
    }

    pub fn devno(mut self, devno: Dev) -> Self {
        self.disk_id = Some(IdType::Devno(devno));
        self
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = offset;
        self
    }

    pub fn flags(mut self, flags: ProbeFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn filter(mut self, filter: ProbeFilter) -> Self {
        self.filter = filter;
        self
    }

    pub fn build(self) -> Result<Probe, BlockidError> {
        let id = self.disk_id.ok_or(BlockidError::ArgumentError(
            "Path/devno not set in ProbeBuilder",
        ))?;

        let (file, path) = match id {
            IdType::Path(path) => (File::open(&path)?, path),
            IdType::Devno(devno) => {
                let path = devno_to_path(devno)?;
                (File::open(&path)?, path)
            }
        };
        Probe::new(file, &path, self.offset, self.flags, self.filter)
    }
}
