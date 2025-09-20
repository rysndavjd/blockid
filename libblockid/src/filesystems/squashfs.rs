use std::io::Error as IoError;

use crate::{
    BlockidError,
    filesystems::FsError,
    probe::{
        BlockType, BlockidIdinfo, BlockidMagic, BlockidVersion, Endianness, FilesystemResult,
        Probe, ProbeResult, UsageType,
    },
};
use rustix::fs::makedev;
use thiserror::Error;
use zerocopy::{FromBytes, Immutable, IntoBytes, Unaligned};

#[derive(Debug, Error)]
pub enum SquashError {
    #[error("I/O operation failed: {0}")]
    IoError(#[from] IoError),
    #[error("Invalid SquashFS version")]
    InvalidSquashVersion,
}

pub const SQUASHFS_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("squashfs"),
    btype: Some(BlockType::Squashfs),
    usage: Some(UsageType::Filesystem),
    probe_fn: |probe, magic| {
        probe_squashfs(probe, magic)
            .map_err(FsError::from)
            .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[BlockidMagic {
        magic: b"hsqs",
        len: 4,
        b_offset: 0,
    }]),
};

pub const SQUASHFS3_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("squashfs3"),
    btype: Some(BlockType::Squashfs3),
    usage: Some(UsageType::Filesystem),
    probe_fn: |probe, magic| {
        probe_squashfs3(probe, magic)
            .map_err(FsError::from)
            .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[
        BlockidMagic {
            magic: b"sqsh",
            len: 4,
            b_offset: 0,
        },
        BlockidMagic {
            magic: b"hsqs",
            len: 4,
            b_offset: 0,
        },
    ]),
};

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct SquashBlock {
    magic: [u8; 4],
    inode_count: [u8; 4],
    mod_time: [u8; 4],
    block_size: [u8; 4],
    frag_count: [u8; 4],
    compressor: [u8; 2],
    block_log: [u8; 2],
    flags: [u8; 2],
    id_count: [u8; 2],
    version_major: [u8; 2],
    version_minor: [u8; 2],
    root_inode: [u8; 8],
    bytes_used: [u8; 8],
    id_table: [u8; 8],
    xattr_table: [u8; 8],
    inode_table: [u8; 8],
    dir_table: [u8; 8],
    frag_table: [u8; 8],
    export_table: [u8; 8],
}

pub fn probe_squashfs(probe: &mut Probe, magic: BlockidMagic) -> Result<(), SquashError> {
    let sb: SquashBlock =
        probe.map_from_file::<{ size_of::<SquashBlock>() }, SquashBlock>(probe.offset())?;

    let vermaj = u16::from_le_bytes(sb.version_major);
    let vermin = u16::from_le_bytes(sb.version_minor);

    if vermaj < 4 {
        return Err(SquashError::InvalidSquashVersion);
    }

    probe.push_result(ProbeResult::Filesystem(FilesystemResult {
        btype: Some(BlockType::Squashfs),
        sec_type: None,
        label: None,
        uuid: None,
        log_uuid: None,
        ext_journal: None,
        creator: None,
        usage: Some(UsageType::Filesystem),
        version: Some(BlockidVersion::DevT(makedev(
            u32::from(vermaj),
            u32::from(vermin),
        ))),
        sbmagic: Some(magic.magic),
        sbmagic_offset: Some(magic.b_offset),
        size: Some(u64::from_le_bytes(sb.bytes_used)),
        fs_last_block: None,
        fs_block_size: Some(u64::from(u32::from_le_bytes(sb.block_size))),
        block_size: Some(u64::from(u32::from_le_bytes(sb.block_size))),
        endianness: None,
    }));

    return Ok(());
}

pub fn probe_squashfs3(probe: &mut Probe, magic: BlockidMagic) -> Result<(), SquashError> {
    let sb: SquashBlock =
        probe.map_from_file::<{ size_of::<SquashBlock>() }, SquashBlock>(probe.offset())?;

    let endianness = if b"sqsh" == &sb.magic {
        Endianness::Big
    } else {
        Endianness::Little
    };

    let vermaj = match endianness {
        Endianness::Big => u16::from_be_bytes(sb.version_major),
        Endianness::Little => u16::from_le_bytes(sb.version_major),
    };
    let vermin = match endianness {
        Endianness::Big => u16::from_be_bytes(sb.version_minor),
        Endianness::Little => u16::from_le_bytes(sb.version_minor),
    };

    if vermaj > 3 {
        return Err(SquashError::InvalidSquashVersion);
    }

    probe.push_result(ProbeResult::Filesystem(FilesystemResult {
        btype: Some(BlockType::Squashfs3),
        sec_type: None,
        label: None,
        uuid: None,
        log_uuid: None,
        ext_journal: None,
        creator: None,
        usage: Some(UsageType::Filesystem),
        version: Some(BlockidVersion::DevT(makedev(
            u32::from(vermaj),
            u32::from(vermin),
        ))),
        sbmagic: Some(magic.magic),
        sbmagic_offset: Some(magic.b_offset),
        size: None,
        fs_last_block: None,
        fs_block_size: Some(1024),
        block_size: Some(1024),
        endianness: Some(endianness),
    }));

    return Ok(());
}
