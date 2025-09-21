use std::{
    io::{Error as IoError, ErrorKind},
    str::FromStr,
};

use crc_fast::{CrcParams, checksum_with_params};
use thiserror::Error;
use uuid::Uuid;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, LittleEndian, Unaligned,
    byteorder::{BigEndian, U16, U32, U64},
};

use crate::{
    BlockidError, Probe,
    containers::ContError,
    probe::{
        BlockType, BlockidIdinfo, BlockidMagic, BlockidUUID, BlockidVersion, ContainerResult,
        ProbeResult, UsageType,
    },
    util::{UtfError, decode_utf8_from},
};

#[derive(Debug, Error)]
pub enum LvmError {
    #[error("I/O operation failed: {0}")]
    IoError(#[from] IoError),
    #[error("Invalid verity hash version")]
    InvalidVerityHashVersion,
}

pub const LVM1_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("LVM1_member"),
    btype: Some(BlockType::Lvm1Member),
    usage: Some(UsageType::Raid),
    probe_fn: |probe, magic| {
        probe_lvm1(probe, magic)
            .map_err(ContError::from)
            .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[BlockidMagic {
        magic: b"HM",
        len: 2,
        b_offset: 0,
    }]),
};

pub const LVM2_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("LVM2_member"),
    btype: Some(BlockType::Lvm2Member),
    usage: Some(UsageType::Raid),
    probe_fn: |probe, magic| {
        probe_lvm2(probe, magic)
            .map_err(ContError::from)
            .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[
        BlockidMagic {
            magic: b"LVM2 001",
            len: 8,
            b_offset: 0x218,
        },
        BlockidMagic {
            magic: b"LVM2 001",
            len: 8,
            b_offset: 0x018,
        },
    ]),
};

pub const LVM_SNAPCOW_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("DM_snapshot_cow"),
    btype: Some(BlockType::LvmSnapcow),
    usage: Some(UsageType::Other("")),
    probe_fn: |probe, magic| {
        probe_snapcow(probe, magic)
            .map_err(ContError::from)
            .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[BlockidMagic {
        magic: b"SnAp",
        len: 4,
        b_offset: 0,
    }]),
};

pub const LVM_VERITY_HASH_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("DM_verity_hash"),
    btype: Some(BlockType::LvmVerityHash),
    usage: Some(UsageType::Crypto),
    probe_fn: |probe, magic| {
        probe_verity_hash(probe, magic)
            .map_err(ContError::from)
            .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[BlockidMagic {
        magic: b"verity\0\0",
        len: 8,
        b_offset: 0,
    }]),
};

pub const LVM_INTEGRITY_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("DM_integrity"),
    btype: Some(BlockType::LvmIntegrity),
    usage: Some(UsageType::Crypto),
    probe_fn: |probe, magic| {
        probe_integrity(probe, magic)
            .map_err(ContError::from)
            .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[BlockidMagic {
        magic: b"integrt\0",
        len: 8,
        b_offset: 0,
    }]),
};

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct Lvm1PvHeader {
    id: [u8; 2],
    version: U16<LittleEndian>,
    unused: [U32<LittleEndian>; 10],
    pv_uuid: [u8; 128],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct Lvm2PvHeader {
    id: [u8; 8],
    sector_xl: U64<LittleEndian>,
    crc_xl: U32<LittleEndian>,
    offset_xl: U32<LittleEndian>,
    pv_type: [u8; 8],
    pv_uuid: [u8; 32],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct VeritySb {
    signature: [u8; 8],
    version: U32<LittleEndian>,
    hash_type: U32<LittleEndian>,
    uuid: [u8; 16],
    algorithm: [u8; 32],
    data_block_size: U32<LittleEndian>,
    hash_block_size: U32<LittleEndian>,
    data_blocks: U64<LittleEndian>,
    salt_size: U16<LittleEndian>,
    pad1: [u8; 6],
    salt: [u8; 256],
    pad2: [u8; 168],
}

pub fn lvm2_crc(buf: &[u8]) -> u64 {
    let lvm2crc = CrcParams::new(
        "LVM2 CRC32",
        32,
        0x1edc6f41,
        0xf597a6cf,
        true,
        0,
        0xe3069283,
    );

    return checksum_with_params(lvm2crc, buf);
}

pub fn probe_lvm1(probe: &mut Probe, _magic: BlockidMagic) -> Result<(), LvmError> {
    return Ok(());
}

pub fn probe_lvm2(probe: &mut Probe, _magic: BlockidMagic) -> Result<(), LvmError> {
    return Ok(());
}

pub fn probe_snapcow(probe: &mut Probe, _magic: BlockidMagic) -> Result<(), LvmError> {
    return Ok(());
}

pub fn probe_verity_hash(probe: &mut Probe, magic: BlockidMagic) -> Result<(), LvmError> {
    let sb = probe.map_from_file::<{ size_of::<VeritySb>() }, VeritySb>(probe.offset())?;

    let version = u64::from(sb.version);

    if version != 1 {
        return Err(LvmError::InvalidVerityHashVersion);
    }

    probe.push_result(ProbeResult::Container(ContainerResult {
        btype: Some(BlockType::LvmVerityHash),
        sec_type: None,
        uuid: Some(Uuid::from_bytes(sb.uuid).into()),
        label: None,
        creator: None,
        usage: Some(UsageType::Crypto),
        version: Some(BlockidVersion::Number(version)),
        sbmagic: Some(magic.magic),
        sbmagic_offset: Some(magic.b_offset),
        endianness: None,
    }));

    return Ok(());
}

pub fn probe_integrity(probe: &mut Probe, _magic: BlockidMagic) -> Result<(), LvmError> {
    return Ok(());
}
