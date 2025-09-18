//0x04C11DB7
use std::{io::Error as IoError, mem::offset_of};

use crate::{
    BlockidError,
    filesystems::FsError,
    probe::{
        BlockType, BlockidIdinfo, BlockidMagic, FilesystemResult, Probe, ProbeResult, UsageType,
    },
    util::decode_utf8_lossy_from,
};
use crc_fast::{CrcAlgorithm::Crc32IsoHdlc, Digest};
use thiserror::Error;
use uuid::Uuid;
use zerocopy::{FromBytes, Immutable, IntoBytes, LittleEndian, U32, U64, Unaligned};

#[derive(Debug, Error)]
pub enum ZoneFsError {
    #[error("I/O operation failed: {0}")]
    IoError(#[from] IoError),
    #[error("Invalid header checksum")]
    HeaderChecksumInvalid,
}

pub const ZONEFS_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("zonefs"),
    btype: Some(BlockType::ZoneFs),
    usage: Some(UsageType::Filesystem),
    probe_fn: |probe, magic| {
        probe_zonefs(probe, magic)
            .map_err(FsError::from)
            .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[BlockidMagic {
        magic: b"SFOZ",
        len: 4,
        b_offset: 0,
    }]),
};

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct ZoneFsBlock {
    s_magic: U32<LittleEndian>,
    s_crc: U32<LittleEndian>,
    s_label: [u8; 32],
    s_uuid: [u8; 16],
    s_features: U64<LittleEndian>,
    s_uid: U32<LittleEndian>,
    s_gid: U32<LittleEndian>,
    s_perm: U32<LittleEndian>,
    s_reserved: [u8; 4020],
}

pub fn probe_zonefs(probe: &mut Probe, magic: BlockidMagic) -> Result<(), ZoneFsError> {
    let sb: ZoneFsBlock = probe.map_from_file(probe.offset())?;
    let bytes = sb.as_bytes();

    let mut digest = Digest::new(Crc32IsoHdlc);

    digest.update(&bytes[..offset_of!(ZoneFsBlock, s_crc)]);
    digest.update(&[0u8; 4]);
    digest.update(&bytes[offset_of!(ZoneFsBlock, s_label)..]);

    let csum = digest.finalize();

    if csum != u64::from(sb.s_crc) {
        return Err(ZoneFsError::HeaderChecksumInvalid);
    }

    let label = if sb.s_label[0] != 0 {
        Some(decode_utf8_lossy_from(&sb.s_label))
    } else {
        None
    };

    probe.push_result(ProbeResult::Filesystem(FilesystemResult {
        btype: Some(BlockType::ZoneFs),
        sec_type: None,
        label,
        uuid: Some(Uuid::from_bytes(sb.s_uuid).into()),
        log_uuid: None,
        ext_journal: None,
        creator: None,
        usage: Some(UsageType::Filesystem),
        version: None,
        sbmagic: Some(magic.magic),
        sbmagic_offset: Some(magic.b_offset),
        size: None,
        fs_last_block: None,
        fs_block_size: Some(4096),
        block_size: Some(4096),
        endianness: None,
    }));

    return Ok(());
}
