use std::io::{Error as IoError, ErrorKind, Read, Seek, SeekFrom};

use bitflags::bitflags;
use thiserror::Error;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned,
    byteorder::big_endian::{U64, U32, U16}, transmute,
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
    checksum::{verify_crc32c, CsumAlgorium}
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
    magicnum: U32,
    blocksize: U32,
    dblocks: U64,
    rblocks: U64,
    rextents: U64,
    uuid: [u8; 16],
    logstart: U64,
    rootino: U64,
    rbmino: U64,
    rsumino: U64,
    rextsize: U32,
    agblocks: U32,
    agcount: U32,
    rbmblocks: U32,
    logblocks: U32,

    versionnum: U16,
    sectsize: U16,
    inodesize: U16,
    inopblock: U16,
    fname: [u8; 12],
    blocklog: u8,
    sectlog: u8,
    inodelog: u8,
    inopblog: u8,
    agblklog: u8,
    rextslog: u8,
    inprogress: u8,
    imax_pct: u8,

    icount: U64,
    ifree: U64,
    fdblocks: U64,
    frextents: U64,
    uquotino: U64,
    gquotino: U64,
    qflags: U16,
    flags: u8,
    shared_vn: u8,
    inoalignmt: U32,
    unit: U32,
    width: U32,
    dirblklog: u8,
    logsectlog: u8,
    logsectsize: U16,
    logsunit: U32,
    features2: U32,
    bad_features2: U32,

    features_compat: U32,
    features_ro_compat: U32,
    features_incompat: U32,
    features_log_incompat: U32,
    crc: U32,
    spino_align: U32,
    pquotino: U64,
    lsn: U64,
    meta_uuid: [u8; 16],
    rrmapino: U64,
}

const XFS_MIN_BLOCKSIZE_LOG: u8 = 9;
const XFS_MAX_BLOCKSIZE_LOG: u8 = 16;
const XFS_MIN_BLOCKSIZE: u32 = 1 << XFS_MIN_BLOCKSIZE_LOG;
const XFS_MAX_BLOCKSIZE: u32 = 1 << XFS_MAX_BLOCKSIZE_LOG;
const XFS_MIN_SECTORSIZE_LOG: u8 = 9;
const XFS_MAX_SECTORSIZE_LOG: u8 = 15;
const XFS_MIN_SECTORSIZE: u16 = 1 << XFS_MIN_SECTORSIZE_LOG;
const XFS_MAX_SECTORSIZE: u16 = 1 << XFS_MAX_SECTORSIZE_LOG;
const XFS_DINODE_MIN_LOG: u8 = 8;
const XFS_DINODE_MAX_LOG: u8 = 11;
const XFS_DINODE_MIN_SIZE: u16 = 1 << XFS_DINODE_MIN_LOG;
const XFS_DINODE_MAX_SIZE: u16 = 1 << XFS_DINODE_MAX_LOG;

const XFS_MAX_RTEXTSIZE: u32 = 1024 * 1024 * 1024;
const XFS_DFL_RTEXTSIZE: u32 = 64 * 1024;
const XFS_MIN_RTEXTSIZE: u32 = 4 * 1024;
const XFS_MIN_AG_BLOCKS: u32 = 64;

fn xfs_max_dblocks(sb: XfsSuperBlock) -> u64 {
    u64::from(sb.agcount.get() * sb.agblocks.get())
}

fn xfs_min_dblocks(sb: XfsSuperBlock) -> u64 {
    u64::from(sb.agcount.get() -1) * u64::from(sb.agblocks.get() + XFS_MIN_AG_BLOCKS)
}

const XFS_SB_VERSION_MOREBITSBIT: u16 = 0x8000;
const XFS_SB_VERSION2_CRCBIT: u32 = 0x00000100;

pub fn xfs_verify(sb: XfsSuperBlock, probe: Probe, mag: BlockidMagic) -> Result<(), XfsError> {
    if sb.agcount.get() == 0 ||
        sb.sectsize.get() < XFS_MIN_SECTORSIZE || 
        sb.sectsize.get() > XFS_MAX_SECTORSIZE || 
        sb.sectlog < XFS_MIN_SECTORSIZE_LOG	 ||
        sb.sectlog >  XFS_MAX_SECTORSIZE_LOG	 ||
        sb.sectsize.get() != (1 << sb.sectlog) || 
        sb.blocksize.get() < XFS_MIN_BLOCKSIZE ||
        sb.blocksize.get() > XFS_MAX_BLOCKSIZE ||
        sb.blocklog < XFS_MIN_BLOCKSIZE_LOG ||
        sb.blocklog > XFS_MAX_BLOCKSIZE_LOG ||
        sb.blocksize.get() != (1 << sb.blocklog) ||
        sb.inodesize.get() < XFS_DINODE_MIN_SIZE ||
        sb.inodesize.get() > XFS_DINODE_MAX_SIZE ||
        sb.inodelog < XFS_DINODE_MIN_LOG ||
        sb.inodelog > XFS_DINODE_MAX_LOG ||
        sb.inodesize != (1 << sb.inodelog) ||
        sb.blocklog - sb.inodelog != sb.inopblog ||
        sb.rextsize * sb.blocksize > XFS_MAX_RTEXTSIZE ||
        sb.rextsize * sb.blocksize < XFS_MIN_RTEXTSIZE ||
        sb.imax_pct > 100 ||
        sb.dblocks == 0 ||
        sb.dblocks.get() > xfs_max_dblocks(sb) ||
        sb.dblocks.get() < xfs_min_dblocks(sb) 
    {

    }

    if (sb.versionnum.get() & 0x0f) == 5 {
        let expected: u32 = sb.crc.get();
        let crc: u32 = 

        if (sb.versionnum.get() & XFS_SB_VERSION_MOREBITSBIT) != 0 {

        };

        if (sb.features2.get() & XFS_SB_VERSION2_CRCBIT) != 0 {

        };

        let mut header_bytes = sb.as_bytes();
        header_bytes[224..228].fill(0);

        if !verify_crc32c(&header_bytes, expected) {
            return Err();
        }

    }
    return Ok(());
}

pub fn xfs_fssize(sb: XfsSuperBlock) -> u64 {
    
}

pub fn probe_xfs(probe: Probe, mag: BlockidMagic) -> Result<(), XfsError> {
    
    
    
    return Ok(());
}
