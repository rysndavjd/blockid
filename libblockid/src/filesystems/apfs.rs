use std::io::Error as IoError;

use crate::{
    BlockidError,
    filesystems::FsError,
    probe::{
        BlockType, BlockidIdinfo, BlockidMagic, FilesystemResult, Probe, ProbeResult, UsageType,
    },
};
use thiserror::Error;
use uuid::Uuid;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, Unaligned, byteorder::LittleEndian, byteorder::U16,
    byteorder::U32, byteorder::U64,
};

#[derive(Debug, Error)]
pub enum ApfsError {
    #[error("I/O operation failed: {0}")]
    IoError(#[from] IoError),
    #[error("Invalid header checksum")]
    HeaderChecksumInvalid,
    #[error("Invalid APFS container superblock type")]
    InvalidSuperblockType,
    #[error("Invalid APFS container superblock subtype")]
    InvalidSuperblockSubType,
    #[error("Padding not zero")]
    PaddingNotZero,
    #[error("Invalid standard block size")]
    InvalidBlockSize,
    #[error("UUID entry is empty")]
    UuidEmpty,
}

const APFS_CONTAINER_SUPERBLOCK_TYPE: u16 = 1;
const APFS_CONTAINER_SUPERBLOCK_SUBTYPE: u16 = 0;
const APFS_STANDARD_BLOCK_SIZE: u32 = 4096;
const APFS_MAGIC: [u8; 4] = *b"NXSB";

pub const APFS_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("apfs"),
    btype: Some(BlockType::Apfs),
    usage: Some(UsageType::Filesystem),
    probe_fn: |probe, magic| {
        probe_apfs(probe, magic)
            .map_err(FsError::from)
            .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[BlockidMagic {
        magic: &APFS_MAGIC,
        len: 4,
        b_offset: 32,
    }]),
};

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct ApfsSuperBlock {
    pub checksum: U64<LittleEndian>,
    pub oid: U64<LittleEndian>,
    pub xid: U64<LittleEndian>,
    pub apfs_type: U16<LittleEndian>,
    pub flags: U16<LittleEndian>,
    pub subtype: U16<LittleEndian>,
    pub pad: U16<LittleEndian>,

    pub magic: [u8; 4],
    pub block_size: U32<LittleEndian>,
    pub block_count: U64<LittleEndian>,
    pub features: U64<LittleEndian>,
    pub read_only_features: U64<LittleEndian>,
    pub incompatible_features: U64<LittleEndian>,
    pub uuid: [u8; 16],
    pub padding: [u8; 4008],
}

pub fn fletcher64(buf: &[u8]) -> u64 {
    let mut lo32: u64 = 0;
    let mut hi32: u64 = 0;

    for i in 0..(buf.len() / 4) {
        let offset = i * 4;
        let word = u32::from_le_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]) as u64;
        lo32 = lo32.wrapping_add(word);
        hi32 = hi32.wrapping_add(lo32);
    }

    let csum_lo = !((lo32.wrapping_add(hi32)) % 0xFFFFFFFF) as u32;
    let csum_hi = !((lo32.wrapping_add(csum_lo as u64)) % 0xFFFFFFFF) as u32;

    return ((csum_hi as u64) << 32) | (csum_lo as u64);
}

pub fn probe_apfs(probe: &mut Probe, _mag: BlockidMagic) -> Result<(), ApfsError> {
    let sb: ApfsSuperBlock =
        probe.map_from_file::<{ size_of::<ApfsSuperBlock>() }, ApfsSuperBlock>(probe.offset())?;

    let csum = fletcher64(&sb.as_bytes()[8..]);

    if u64::from(sb.checksum) != csum {
        return Err(ApfsError::HeaderChecksumInvalid);
    }

    if u16::from(sb.apfs_type) != APFS_CONTAINER_SUPERBLOCK_TYPE {
        return Err(ApfsError::InvalidSuperblockType);
    }

    if u16::from(sb.subtype) != APFS_CONTAINER_SUPERBLOCK_SUBTYPE {
        return Err(ApfsError::InvalidSuperblockSubType);
    }

    if u16::from(sb.pad) != 0 {
        return Err(ApfsError::PaddingNotZero);
    }

    if u32::from(sb.block_size) != APFS_STANDARD_BLOCK_SIZE {
        return Err(ApfsError::InvalidBlockSize);
    }

    let uuid = if sb.uuid != [0u8; 16] {
        Uuid::from_bytes(sb.uuid)
    } else {
        return Err(ApfsError::UuidEmpty);
    };

    probe.push_result(ProbeResult::Filesystem(FilesystemResult {
        btype: Some(BlockType::Apfs),
        sec_type: None,
        uuid: Some(uuid.into()),
        log_uuid: None,
        ext_journal: None,
        label: None,
        creator: None,
        usage: Some(UsageType::Filesystem),
        size: Some(u64::from(sb.block_count) * u64::from(sb.block_size)), // Dont know if this is correct
        fs_last_block: None,
        fs_block_size: Some(u64::from(sb.block_size)),
        block_size: Some(u64::from(sb.block_size)),
        version: None,
        sbmagic: Some(&APFS_MAGIC),
        sbmagic_offset: Some(32),
        endianness: None,
    }));

    return Ok(());
}
