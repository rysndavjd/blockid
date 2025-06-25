use core::str;
use std::{io::{self, Read, Seek}, str::FromStr};

use bytemuck::{Pod, Zeroable};
use thiserror::Error;
use uuid::{Uuid};
use std::str::Utf8Error;

use crate::{
    containers::ContError, read_as, BlockidError, BlockidIdinfo, 
    BlockidMagic, BlockidProbe, BlockidUUID, BlockidVersion, 
    ContainerResults, ProbeResult, UsageType
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
    #[error("Converting uuid from disk failed: {0}")]
    UuidConversionError(#[from] uuid::Error),
    #[error("UTF-8 error: {0}")]
    UTF8ErrorError(#[from] Utf8Error),
    #[error("Luks Header Error: {0}")]
    LuksHeaderError(&'static str),
    #[error("Not an LUKS superblock: {0}")]
    UnknownFilesystem(&'static str),
}

impl From<LuksError> for ContError {
    fn from(err: LuksError) -> Self {
        match err {
            LuksError::IoError(e) => ContError::IoError(e),
            LuksError::UuidConversionError(_) => ContError::InvalidHeader("Invalid string to convert to uuid"),
            LuksError::UTF8ErrorError(_) => ContError::InvalidHeader("Invalid utf8 to convert to string"),
            LuksError::LuksHeaderError(info) => ContError::InvalidHeader(info),
            LuksError::UnknownFilesystem(info) => ContError::UnknownContainer(info),
        }
    }
}

pub const LUKS1_MAGIC: [u8; 6] = *b"LUKS\xba\xbe";
pub const LUKS2_MAGIC: [u8; 6] = *b"SKUL\xba\xbe";
pub const LUKS2_HW_OPAL_SUBSYSTEM: [u8; 7] = *b"HW-OPAL";

pub const SECONDARY_OFFSETS: [u64; 9] = [0x04000, 0x008000, 0x010000, 0x020000,
                             0x40000, 0x080000, 0x100000, 0x200000, 0x400000];

pub const LUKS1_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("crypto_LUKS"),
    usage: Some(UsageType::Crypto),
    probe_fn: |probe, magic| {
        probe_luks1(probe, magic)
        .map_err(ContError::from)
        .map_err(BlockidError::from)
    },
    minsz: None,
    magics: &[
        BlockidMagic {
            magic: &LUKS1_MAGIC,
            len: 6,
            b_offset: 0,
        },
    ]
};

pub const LUKS2_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("LUKS2"),
    usage: Some(UsageType::Crypto),
    probe_fn: |probe, magic| {
        probe_luks2(probe, magic)
        .map_err(ContError::from)
        .map_err(BlockidError::from)
    },
    minsz: None,
    magics: &[
        BlockidMagic {
            magic: &LUKS2_MAGIC,
            len: 6,
            b_offset: 0,
        },
    ]
};

pub const LUKS_OPAL_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("LUKS OPAL"),
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

impl Luks1Header {
    fn get_uuid(
            self
        ) -> Result<Uuid, LuksError>
    {
        let uuid_str = str::from_utf8(&self.uuid)?;
        let uuid = Uuid::from_str(&uuid_str.trim_end_matches('\0'))?;

        return Ok(uuid);
    }

    fn luks_valid(
            self,
        ) -> bool
    {
        if self.magic == LUKS1_MAGIC &&
            u16::from_be(self.version) == 1
        {
            return true;
        }

        return false;
    }
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

impl Luks2Header {
    fn get_uuid(
            self
        ) -> Result<Uuid, LuksError>
    {
        let uuid_str = str::from_utf8(&self.uuid)?;
        let uuid = Uuid::from_str(&uuid_str.trim_end_matches('\0'))?;

        return Ok(uuid);
    }

    fn luks_valid<R: Seek+Read>(
            self,
            file: &mut R,
        ) -> bool
    {
        if self.magic == LUKS1_MAGIC && u16::from_be(self.version) == 2 {
            return true;
        }
        
        for offset in SECONDARY_OFFSETS {
            match read_as::<Luks2Header, R>(file, offset) {
                Ok(secondary) => {
                    if u16::from_be(secondary.version) == 2 && 
                        u64::from_be(secondary.hdr_offset) == offset 
                    {
                        return true;
                    }
                },
                Err(_) => return false,
            };
        }

        return false;
    }
}

pub fn probe_luks1(
        probe: &mut BlockidProbe, 
        _magic: BlockidMagic
    ) -> Result<(), LuksError> 
{
    let header: Luks1Header = read_as(&mut probe.file, probe.offset)?;
    
    if !header.luks_valid() {
        return Err(LuksError::LuksHeaderError("Luks is not valid luks1 container"));
    }

    probe.push_result(ProbeResult::Container(
                ContainerResults { 
                    cont_type: Some(crate::ContType::LUKS1), 
                    label: None, 
                    cont_uuid: Some(BlockidUUID::Standard(header.get_uuid()?)), 
                    cont_creator: None, 
                    usage: Some(UsageType::Crypto), 
                    version: Some(BlockidVersion::Number(u16::from_be(header.version) as u64)), 
                    sbmagic: Some(&LUKS1_MAGIC), 
                    sbmagic_offset: Some(0), 
                    cont_size: None, 
                    cont_block_size: None 
                }
            )
        );

    return Ok(());
}

pub fn probe_luks2(
        probe: &mut BlockidProbe, 
        _magic: BlockidMagic
    ) -> Result<(), LuksError> 
{
    let header: Luks2Header = read_as(&mut probe.file, probe.offset)?;

    if !header.luks_valid(&mut probe.file) {
        return Err(LuksError::LuksHeaderError("Luks is not valid luks2 container"));
    }

    probe.push_result(ProbeResult::Container(
                ContainerResults { 
                    cont_type: Some(crate::ContType::LUKS1), 
                    label: None, 
                    cont_uuid: Some(BlockidUUID::Standard(header.get_uuid()?)), 
                    cont_creator: None, 
                    usage: Some(UsageType::Crypto), 
                    version: Some(BlockidVersion::Number(u16::from_be(header.version) as u64)), 
                    sbmagic: Some(&LUKS2_MAGIC), 
                    sbmagic_offset: Some(0), 
                    cont_size: None, 
                    cont_block_size: None 
                }
            )
        );

    return Ok(());
}

pub fn probe_luks_opal(
        probe: &mut BlockidProbe, 
        _magic: BlockidMagic
    ) -> Result<(), LuksError> 
{
    let header: Luks2Header = read_as(&mut probe.file, probe.offset)?;

    if !header.luks_valid(&mut probe.file) {
        return Err(LuksError::LuksHeaderError("Luks is not valid luks2 opal container"));
    }

    if header.subsystem[0..7] == LUKS2_HW_OPAL_SUBSYSTEM {
        return Err(LuksError::LuksHeaderError("Luks2 does not contain opal subsystem to be opal"));
    }

    // TODO probe_is_opal_locked

    probe.push_result(ProbeResult::Container(
                ContainerResults { 
                    cont_type: Some(crate::ContType::LUKS1), 
                    label: None, 
                    cont_uuid: Some(BlockidUUID::Standard(header.get_uuid()?)), 
                    cont_creator: None, 
                    usage: Some(UsageType::Crypto), 
                    version: Some(BlockidVersion::Number(u16::from_be(header.version) as u64)), 
                    sbmagic: Some(&LUKS2_MAGIC), 
                    sbmagic_offset: Some(0), 
                    cont_size: None, 
                    cont_block_size: None 
                }
            )
        );

    return Ok(());
}