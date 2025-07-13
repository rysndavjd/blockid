use core::fmt;
use alloc::vec::Vec;

#[cfg(feature = "std")]
use std::io::{Error as IoError, Seek, Read};

#[cfg(not(feature = "std"))]
use crate::nostd_io::{NoStdIoError as IoError, Read, Seek};

use bitflags::bitflags;
use zerocopy::{FromBytes, IntoBytes, Unaligned, 
    byteorder::U32, byteorder::LittleEndian,
    transmute, Immutable};

use crate::{
    BlockidError, BlockidIdinfo, BlockidMagic, BlockidProbe, BlockidUUID,
    PartEntryAttributes, PartEntryType, PartTableResults, PartitionResults,
    ProbeResult, PtType, UsageType, from_file, read_sector_at, filesystems::{
    volume_id::VolumeId32}, partitions::{PtError},
};

#[derive(Debug)]
pub enum GptPtError {
    IoError(IoError),
    UnknownPartitionTable(&'static str),
    DosPTHeaderError(&'static str),
}

impl fmt::Display for GptPtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GptPtError::IoError(e) => write!(f, "I/O operation failed: {}", e),
            GptPtError::UnknownPartitionTable(e) => write!(f, "Not an GPT table superblock: {}", e),
            GptPtError::DosPTHeaderError(e) => write!(f, "GPT table header error: {}", e),
        }
    }
}

impl From<GptPtError> for PtError {
    fn from(err: GptPtError) -> Self {
        match err {
            GptPtError::IoError(e) => PtError::IoError(e),
            GptPtError::UnknownPartitionTable(pt) => PtError::UnknownPartition(pt),
            GptPtError::DosPTHeaderError(pt) => PtError::InvalidHeader(pt),
        }
    }
}

impl From<IoError> for GptPtError {
    fn from(err: IoError) -> Self {
        GptPtError::IoError(err)
    }
}

pub const GPT_PT_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("gpt"),
    usage: Some(UsageType::PartitionTable),
    minsz: None,
    probe_fn: |probe, magic| {
        probe_gpt_pt(probe, magic)
        .map_err(PtError::from)
        .map_err(BlockidError::from)
    },
    magics: None
};

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct GptTable {
}