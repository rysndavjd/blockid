use std::{
    io::{Error as IoError, ErrorKind, Read, Seek},
    str::FromStr,
};

#[cfg(not(target_os = "linux"))]
use log::warn;
use thiserror::Error;
use uuid::Uuid;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, Unaligned, byteorder::BigEndian, byteorder::U16,
    byteorder::U32, byteorder::U64,
};

use crate::{
    BlockidError, Probe,
    containers::ContError,
    probe::{
        BlockType, BlockidIdinfo, BlockidMagic, BlockidUUID, BlockidVersion, ContainerResult,
        ProbeResult, UsageType,
    },
    util::{UtfError, decode_utf8_from, from_file},
};

/*
 * https://en.wikipedia.org/wiki/Linux_Unified_Key_Setup#LUKS2
 * https://cdn.kernel.org/pub/linux/utils/cryptsetup/LUKS_docs/on-disk-format.pdf
 * https://gitlab.com/cryptsetup/LUKS2-docs
*/

#[derive(Debug, Error)]
pub enum LuksError {
    #[error("I/O operation failed: {0}")]
    IoError(#[from] IoError),
    #[error("I/O operation failed: {0}")]
    UuidConversionError(#[from] uuid::Error),
    #[error("UTF operation failed: {0}")]
    UtfError(#[from] UtfError),
    #[error("*Nix operation failed: {0}")]
    NixError(#[from] rustix::io::Errno),
    #[error("Invalid LUKS1 header")]
    InvalidLuksOne,
    #[error("Invalid LUKS2 header")]
    InvalidLuksTwo,
    #[error("Invalid LUKS2 Opal header")]
    InvalidLuksTwoOpal,
}

pub const LUKS1_MAGIC: [u8; 6] = *b"LUKS\xba\xbe";
pub const LUKS2_MAGIC: [u8; 6] = *b"SKUL\xba\xbe";
pub const LUKS2_HW_OPAL_SUBSYSTEM: [u8; 7] = *b"HW-OPAL";

pub const SECONDARY_OFFSETS: [u64; 9] = [
    0x04000, 0x008000, 0x010000, 0x020000, 0x40000, 0x080000, 0x100000, 0x200000, 0x400000,
];

pub const LUKS1_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("luks1"),
    btype: Some(BlockType::LUKS1),
    usage: Some(UsageType::Crypto),
    probe_fn: |probe, magic| {
        probe_luks1(probe, magic)
            .map_err(ContError::from)
            .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[BlockidMagic {
        magic: &LUKS1_MAGIC,
        len: 6,
        b_offset: 0,
    }]),
};

pub const LUKS2_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("luks1"),
    btype: Some(BlockType::LUKS2),
    usage: Some(UsageType::Crypto),
    probe_fn: |probe, magic| {
        probe_luks2(probe, magic)
            .map_err(ContError::from)
            .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[BlockidMagic {
        magic: &LUKS2_MAGIC,
        len: 6,
        b_offset: 0,
    }]),
};

pub const LUKS_OPAL_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("LUKS_OPAL"),
    btype: Some(BlockType::LUKSOpal),
    usage: Some(UsageType::Crypto),
    probe_fn: |probe, magic| {
        probe_luks_opal(probe, magic)
            .map_err(ContError::from)
            .map_err(BlockidError::from)
    },
    minsz: None,
    magics: None,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct Luks1Header {
    pub magic: [u8; 6],
    pub version: U16<BigEndian>,
    pub cipher_name: [u8; 32],
    pub cipher_mode: [u8; 32],
    pub hash_spec: [u8; 32],
    pub payload_offset: U32<BigEndian>,
    pub key_bytes: U32<BigEndian>,
    pub mk_digest: [u8; 20],
    pub mk_digest_salt: [u8; 32],
    pub mk_digest_iterations: U32<BigEndian>,
    pub uuid: [u8; 40],
}

impl Luks1Header {
    fn luks_valid(self) -> bool {
        if self.magic == LUKS1_MAGIC && u16::from(self.version) == 1 {
            return true;
        }

        return false;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct Luks2Header {
    pub magic: [u8; 6],
    pub version: U16<BigEndian>,
    pub hdr_size: U64<BigEndian>,
    pub seqid: U64<BigEndian>,
    pub label: [u8; 48],
    pub checksum_alg: [u8; 32],
    pub salt: [u8; 64],
    pub uuid: [u8; 40],
    pub subsystem: [u8; 48],
    pub hdr_offset: U64<BigEndian>,
    _padding: [u8; 184],
    pub csum: [u8; 64],
}

impl Luks2Header {
    fn luks_valid<R: Seek + Read>(self, file: &mut R) -> bool {
        if self.magic == LUKS1_MAGIC && u16::from(self.version) == 2 {
            return true;
        }

        for offset in SECONDARY_OFFSETS {
            match from_file::<Luks2Header, R>(file, offset) {
                Ok(secondary) => {
                    if u16::from(secondary.version) == 2
                        && u64::from(secondary.hdr_offset) == offset
                    {
                        return true;
                    }
                }
                Err(_) => return false,
            };
        }

        return false;
    }
}

pub fn probe_luks1(probe: &mut Probe, _magic: BlockidMagic) -> Result<(), LuksError> {
    let header: Luks1Header = from_file(&mut probe.file(), probe.offset())?;

    if !header.luks_valid() {
        return Err(LuksError::InvalidLuksOne);
    }

    probe.push_result(ProbeResult::Container(ContainerResult {
        btype: Some(BlockType::LUKS1),
        sec_type: None,
        label: None,
        uuid: Some(BlockidUUID::Uuid(Uuid::from_str(&decode_utf8_from(
            &header.uuid,
        )?)?)),
        creator: None,
        usage: Some(UsageType::Crypto),
        version: Some(BlockidVersion::Number(u64::from(header.version))),
        sbmagic: Some(&LUKS1_MAGIC),
        sbmagic_offset: Some(0),
        endianness: None,
    }));
    return Ok(());
}

pub fn probe_luks2(probe: &mut Probe, _magic: BlockidMagic) -> Result<(), LuksError> {
    let header: Luks2Header = from_file(&mut probe.file(), probe.offset())?;

    if !header.luks_valid(&mut probe.file()) {
        return Err(LuksError::InvalidLuksTwo);
    }

    probe.push_result(ProbeResult::Container(ContainerResult {
        btype: Some(BlockType::LUKS2),
        sec_type: None,
        label: None,
        uuid: Some(BlockidUUID::Uuid(Uuid::from_str(&decode_utf8_from(
            &header.uuid,
        )?)?)),
        creator: None,
        usage: Some(UsageType::Crypto),
        version: Some(BlockidVersion::Number(u64::from(header.version))),
        sbmagic: Some(&LUKS2_MAGIC),
        sbmagic_offset: Some(0),
        endianness: None,
    }));
    return Ok(());
}

pub fn probe_luks_opal(probe: &mut Probe, _magic: BlockidMagic) -> Result<(), LuksError> {
    let header: Luks2Header = from_file(&mut probe.file(), probe.offset())?;

    if !header.luks_valid(&mut probe.file()) {
        return Err(LuksError::InvalidLuksTwoOpal);
    }

    if header.subsystem[0..7] == LUKS2_HW_OPAL_SUBSYSTEM {
        return Err(LuksError::InvalidLuksTwoOpal);
    }

    #[cfg(target_os = "linux")]
    if probe.is_opal_locked()? {
        return Err(LuksError::IoError(ErrorKind::PermissionDenied.into()));
    }
    #[cfg(not(target_os = "linux"))]
    warn!(
        "Unable to check if opal is locked as the ioctl call is unavilable on non-linux platforms"
    );

    probe.push_result(ProbeResult::Container(ContainerResult {
        btype: Some(BlockType::LUKSOpal),
        sec_type: None,
        label: None,
        uuid: Some(BlockidUUID::Uuid(Uuid::from_str(&decode_utf8_from(
            &header.uuid,
        )?)?)),
        creator: None,
        usage: Some(UsageType::Crypto),
        version: Some(BlockidVersion::Number(u64::from(header.version))),
        sbmagic: Some(&LUKS1_MAGIC),
        sbmagic_offset: Some(0),
        endianness: None,
    }));
    return Ok(());
}
