use core::str;
use std::io::{self, BufRead, BufReader, Read, Seek};

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use rustix::fs::makedev;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    checksum::{get_crc32c, verify_crc32c, CsumAlgorium}, containers::ContError, read_as, read_buffer, BlockidError, BlockidIdinfo, BlockidMagic, BlockidProbe, BlockidUUID, BlockidVersion, FilesystemResults, FsType, ProbeResult, UsageType
};

/* 
 * https://en.wikipedia.org/wiki/Linux_Unified_Key_Setup#LUKS2
 * https://cdn.kernel.org/pub/linux/utils/cryptsetup/LUKS_docs/on-disk-format.pdf
 * https://gitlab.com/cryptsetup/LUKS2-docs
*/

#[derive(Error, Debug)]
pub enum LuksError {
    #[error("I/O operation failed: {0}")]
    IoError(#[from] io::Error),
    #[error("Not an LUKS superblock: {0}")]
    UnknownFilesystem(&'static str),
    #[error("{algorium} checksum failed, expected: \"{expected:X}\" and got: \"{got:X})\"")]
    ChecksumError {
        algorium: String,
        expected: CsumAlgorium,
        got: CsumAlgorium,
    }
}

pub const LUKS1_MAGIC: [u8; 6] = *b"LUKS\xba\xbe";
pub const LUKS2_MAGIC: [u8; 6] = *b"SKUL\xba\xbe";

pub const LUKS_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("crypto_LUKS"),
    usage: Some(UsageType::Crypto),
    probe_fn: |probe, magic| {
        probe_luks(probe, magic)
        .map_err(ContError::from)
        .map_err(BlockidError::from)
    },
    minsz: None,
    magics: &[
        BlockidMagic {
            magic: &LUKS1_MAGIC,
            len: 2,
            b_offset: 0,
        },
        BlockidMagic {
            magic: &LUKS2_MAGIC,
            len: 6,
            b_offset: 0,
        },
    ]
};

pub const LUKS_OPAL_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("crypto_LUKS"),
    usage: Some(UsageType::Crypto),
    probe_fn: |probe, magic| {
        probe_luks_opal(probe, magic)
        .map_err(ContError::from)
        .map_err(BlockidError::from)
    },
    minsz: None,
    magics: &[]
};

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Luks1Header {
    pub magic: [u8; 6],
    pub version: u16,
    pub cipher_name: [u8; 32],
    pub cipher_mode: [u8; 32],
    pub hash_spec: [u8; 32],
    pub payload_offset: u32,
    pub key_bytes: u32,
    pub mk_digest: [u8; 20],
    pub mk_digest_salt: [u8; 32],
    pub mk_digest_iterations: u32,
    pub uuid: [u8; 40],
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Luks2Header {
    pub magic: [u8; 6],
    pub version: u16,
    pub hdr_size: u64,
    pub seqid: u64,
    pub label: [u8; 48],
    pub checksum_alg: [u8; 32],
    pub salt: [u8; 64],
    pub uuid: [u8; 40],
    pub subsystem: [u8; 48],
    pub hdr_offset: u64,
    _padding: [u8; 184],
    pub csum: [u8; 64],
}

pub fn probe_luks(
        probe: &mut BlockidProbe, 
        _magic: BlockidMagic
    ) -> Result<ProbeResult, LuksError> 
{
    let mut buffer = BufReader::with_capacity(8096, &probe.file);



    Ok(())
}

pub fn probe_luks_opal(
        probe: &mut BlockidProbe, 
        _magic: BlockidMagic
    ) -> Result<ProbeResult, LuksError> 
{
    
}