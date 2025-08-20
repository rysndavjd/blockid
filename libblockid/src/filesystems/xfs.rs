use std::io::{Error as IoError, ErrorKind, Read, Seek, SeekFrom};

use bitflags::bitflags;
use thiserror::Error;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, byteorder::LittleEndian,
    byteorder::U16, byteorder::U32, transmute,
};

use crate::{
    BlockidError,
    filesystems::FsError,
    probe::{
        BlockType, BlockidIdinfo, BlockidMagic, BlockidUUID, FilesystemResult, Probe, ProbeResult,
        SecType, UsageType,
    },
    util::{
        decode_utf8_lossy_from, from_file, is_power_2, probe_get_magic, read_exact_at, read_vec_at,
    },
};

#[derive(Debug, Error)]
pub enum XfsError {
    #[error("I/O operation failed: {0}")]
    IoError(IoError),
    #[error("Xfs Header Error: {0}")]
    XfsHeaderError(&'static str),
    #[error("Unknown FS: {0}")]
    UnknownFilesystem(&'static str),
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
struct XfsSuperBlock {
    magicnum: [u8; 4],
    blocksize: [u8; 4],
    dblocks: [u8; 8],
    rblocks: [u8; 8],
    rextents: [u8; 8],
    uuid: [u8; 16],
    logstart: [u8; 8],
    rootino: [u8; 8],
    rbmino: [u8; 8],
    rsumino: [u8; 8],
    rextsize: [u8; 4],
    agblocks: [u8; 4],
    agcount: [u8; 4],
    rbmblocks: [u8; 4],
    logblocks: [u8; 4],

    versionnum: [u8; 2],
    sectsize: [u8; 2],
    inodesize: [u8; 2],
    inopblock: [u8; 2],
    fname: [u8; 12],
    blocklog: u8,
    sectlog: u8,
    inodelog: u8,
    inopblog: u8,
    agblklog: u8,
    rextslog: u8,
    inprogress: u8,
    imax_pct: u8,

    icount: [u8; 8],
    ifree: [u8; 8],
    fdblocks: [u8; 8],
    frextents: [u8; 8],
    uquotino: [u8; 8],
    gquotino: [u8; 8],
    qflags: [u8; 2],
    flags: u8,
    shared_vn: u8,
    inoalignmt: [u8; 4],
    unit: [u8; 4],
    width: [u8; 4],
    dirblklog: u8,
    logsectlog: u8,
    logsectsize: [u8; 2],
    logsunit: [u8; 4],
    features2: [u8; 4],
    bad_features2: [u8; 4],

    features_compat: [u8; 4],
    features_ro_compat: [u8; 4],
    features_incompat: [u8; 4],
    features_log_incompat: [u8; 4],
    crc: [u8; 4],
    spino_align: [u8; 4],
    pquotino: [u8; 8],
    lsn: [u8; 8],
    meta_uuid: [u8; 16],
    rrmapino: [u8; 8],
}

pub fn probe_xfs(probe: Probe, mag: BlockidMagic) -> Result<(), XfsError> {
    return Ok(());
}
