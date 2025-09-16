use std::{io::Error as IoError, mem::offset_of};

use crc_fast::{CrcAlgorithm::Crc32Iscsi, Digest};
use thiserror::Error;
use uuid::Uuid;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, byteorder::BigEndian, byteorder::U16,
    byteorder::U32, byteorder::U64,
};

use crate::{
    BlockidError,
    filesystems::FsError,
    probe::{
        BlockType, BlockidIdinfo, BlockidMagic, BlockidUUID, FilesystemResult, Probe, ProbeResult,
        UsageType,
    },
    util::decode_utf8_lossy_from,
};

#[derive(Debug, Error)]
pub enum XfsError {
    #[error("I/O operation failed: {0}")]
    IoError(#[from] IoError),
    #[error("Invalid XFS header ranges")]
    InvalidHeaderRanges,
    #[error("Invalid XFS header version number")]
    InvalidHeaderVersion,
    #[error("Invalid XFS header features")]
    InvalidHeaderFeatures,
    #[error("Invalid header checksum")]
    HeaderChecksumInvalid,
}

pub const XFS_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("xfs"),
    btype: Some(BlockType::Xfs),
    usage: Some(UsageType::Filesystem),
    probe_fn: |probe, magic| {
        probe_xfs(probe, magic)
            .map_err(FsError::from)
            .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[BlockidMagic {
        magic: b"XFSB",
        len: 4,
        b_offset: 0,
    }]),
};

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout)]
pub struct XfsSuperBlock {
    magicnum: U32<BigEndian>,
    blocksize: U32<BigEndian>,
    dblocks: U64<BigEndian>,
    rblocks: U64<BigEndian>,
    rextents: U64<BigEndian>,
    uuid: [u8; 16],
    logstart: U64<BigEndian>,
    rootino: U64<BigEndian>,
    rbmino: U64<BigEndian>,
    rsumino: U64<BigEndian>,
    rextsize: U32<BigEndian>,
    agblocks: U32<BigEndian>,
    agcount: U32<BigEndian>,
    rbmblocks: U32<BigEndian>,
    logblocks: U32<BigEndian>,

    versionnum: U16<BigEndian>,
    sectsize: U16<BigEndian>,
    inodesize: U16<BigEndian>,
    inopblock: U16<BigEndian>,
    fname: [u8; 12],
    blocklog: u8,
    sectlog: u8,
    inodelog: u8,
    inopblog: u8,
    agblklog: u8,
    rextslog: u8,
    inprogress: u8,
    imax_pct: u8,

    icount: U64<BigEndian>,
    ifree: U64<BigEndian>,
    fdblocks: U64<BigEndian>,
    frextents: U64<BigEndian>,
    uquotino: U64<BigEndian>,
    gquotino: U64<BigEndian>,
    qflags: U16<BigEndian>,
    flags: u8,
    shared_vn: u8,
    inoalignmt: U32<BigEndian>,
    unit: U32<BigEndian>,
    width: U32<BigEndian>,
    dirblklog: u8,
    logsectlog: u8,
    logsectsize: U16<BigEndian>,
    logsunit: U32<BigEndian>,
    features2: U32<BigEndian>,
    bad_features2: U32<BigEndian>,

    features_compat: U32<BigEndian>,
    features_ro_compat: U32<BigEndian>,
    features_incompat: U32<BigEndian>,
    features_log_incompat: U32<BigEndian>,
    crc: U32<BigEndian>,
    spino_align: U32<BigEndian>,
    pquotino: U64<BigEndian>,
    lsn: U64<BigEndian>,
    meta_uuid: [u8; 16],
    rrmapino: U64<BigEndian>,
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
//const XFS_DFL_RTEXTSIZE: u32 = 64 * 1024;
const XFS_MIN_RTEXTSIZE: u32 = 4 * 1024;
const XFS_MIN_AG_BLOCKS: u32 = 64;

fn xfs_max_dblocks(sb: XfsSuperBlock) -> u64 {
    u64::from(sb.agcount.get() * sb.agblocks.get())
}

fn xfs_min_dblocks(sb: XfsSuperBlock) -> u64 {
    u64::from(sb.agcount.get() - 1) * u64::from(sb.agblocks.get() + XFS_MIN_AG_BLOCKS)
}

const XFS_SB_VERSION_MOREBITSBIT: u16 = 0x8000;
const XFS_SB_VERSION2_CRCBIT: u32 = 0x00000100;

pub fn xfs_verify(sb: XfsSuperBlock, crc_area: Vec<u8>) -> Result<(), XfsError> {
    if sb.agcount.get() == 0
        || sb.sectsize.get() < XFS_MIN_SECTORSIZE
        || sb.sectsize.get() > XFS_MAX_SECTORSIZE
        || sb.sectlog < XFS_MIN_SECTORSIZE_LOG
        || sb.sectlog > XFS_MAX_SECTORSIZE_LOG
        || sb.sectsize.get() != (1 << sb.sectlog)
        || sb.blocksize.get() < XFS_MIN_BLOCKSIZE
        || sb.blocksize.get() > XFS_MAX_BLOCKSIZE
        || sb.blocklog < XFS_MIN_BLOCKSIZE_LOG
        || sb.blocklog > XFS_MAX_BLOCKSIZE_LOG
        || sb.blocksize.get() != (1 << sb.blocklog)
        || sb.inodesize.get() < XFS_DINODE_MIN_SIZE
        || sb.inodesize.get() > XFS_DINODE_MAX_SIZE
        || sb.inodelog < XFS_DINODE_MIN_LOG
        || sb.inodelog > XFS_DINODE_MAX_LOG
        || sb.inodesize != (1 << sb.inodelog)
        || sb.blocklog - sb.inodelog != sb.inopblog
        || sb.rextsize * sb.blocksize > XFS_MAX_RTEXTSIZE
        || sb.rextsize * sb.blocksize < XFS_MIN_RTEXTSIZE
        || sb.imax_pct > 100
        || sb.dblocks == 0
        || sb.dblocks.get() > xfs_max_dblocks(sb)
        || sb.dblocks.get() < xfs_min_dblocks(sb)
    {
        return Err(XfsError::InvalidHeaderRanges);
    }

    if (sb.versionnum.get() & 0x0f) == 5 {
        if (sb.versionnum.get() & XFS_SB_VERSION_MOREBITSBIT) == 0 {
            return Err(XfsError::InvalidHeaderVersion);
        };

        if (sb.features2.get() & XFS_SB_VERSION2_CRCBIT) == 0 {
            return Err(XfsError::InvalidHeaderFeatures);
        };

        let mut digest = Digest::new(Crc32Iscsi);

        digest.update(&crc_area[0..offset_of!(XfsSuperBlock, crc)]);
        digest.update(&[0u8; 4]);
        digest.update(&crc_area[offset_of!(XfsSuperBlock, spino_align)..]);

        let crc_bytes = digest.finalize().to_le_bytes();

        if sb.crc.as_bytes() != [crc_bytes[0], crc_bytes[1], crc_bytes[2], crc_bytes[3]] {
            return Err(XfsError::HeaderChecksumInvalid);
        }
    }
    return Ok(());
}

pub fn xfs_fssize(sb: XfsSuperBlock) -> u64 {
    let lsize = if sb.logstart.get() != 0 {
        sb.logblocks.get()
    } else {
        0
    };

    let avail_blocks = sb.dblocks.get() - u64::from(lsize);
    let fssize = avail_blocks * u64::from(sb.blocksize.get());

    return fssize;
}

pub fn probe_xfs(probe: &mut Probe, _mag: BlockidMagic) -> Result<(), XfsError> {
    let sb: XfsSuperBlock = probe.map_from_file(probe.offset())?;
    let crc_area = probe.read_vec_at(probe.offset(), usize::from(sb.sectsize))?;

    xfs_verify(sb, crc_area)?;

    let label = if sb.fname[0] != 0 {
        Some(decode_utf8_lossy_from(&sb.fname))
    } else {
        None
    };

    probe.push_result(ProbeResult::Filesystem(FilesystemResult {
        btype: Some(BlockType::Xfs),
        sec_type: None,
        uuid: Some(BlockidUUID::Uuid(Uuid::from_bytes(sb.uuid))),
        log_uuid: None,
        ext_journal: None,
        label,
        creator: None,
        usage: Some(UsageType::Filesystem),
        size: Some(xfs_fssize(sb)),
        fs_last_block: Some(sb.dblocks.get()),
        fs_block_size: Some(u64::from(sb.blocksize.get())),
        block_size: Some(u64::from(sb.sectsize.get())),
        version: None,
        sbmagic: Some(b"XFSB"),
        sbmagic_offset: Some(0),
        endianness: None,
    }));
    return Ok(());
}
