use core::fmt::{self, Debug};
use alloc::str::{Utf8Error, FromStr};

#[cfg(feature = "std")]
use std::{io::{Error as IoError, Read, Seek, ErrorKind}};

#[cfg(not(feature = "std"))]
use crate::nostd_io::{NoStdIoError as IoError, Read, Seek, ErrorKind};

use zerocopy::{FromBytes, IntoBytes, Unaligned, 
    byteorder::U64, byteorder::U32, byteorder::U16, 
    byteorder::BigEndian, Immutable};
use uuid::{Uuid};

use crate::{
    containers::ContError, from_file, BlockidError, BlockidIdinfo, 
    BlockidMagic, BlockidProbe, BlockidUUID, BlockidVersion, 
    ContainerResults, ProbeResult, UsageType, Endianness
};

/* 
 * https://en.wikipedia.org/wiki/Linux_Unified_Key_Setup#LUKS2
 * https://cdn.kernel.org/pub/linux/utils/cryptsetup/LUKS_docs/on-disk-format.pdf
 * https://gitlab.com/cryptsetup/LUKS2-docs
*/

#[derive(Debug)]
pub enum LuksError {
    IoError(IoError),
    UuidConversionError(uuid::Error),
    UTF8ErrorError(Utf8Error),
    LuksHeaderError(&'static str),
    UnknownFilesystem(&'static str),
    NixError(rustix::io::Errno),

}

impl fmt::Display for LuksError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LuksError::IoError(e) => write!(f, "I/O operation failed: {e}"),
            LuksError::UuidConversionError(e) => write!(f, "Converting uuid from disk failed: {e}"),
            LuksError::UTF8ErrorError(e) => write!(f, "UTF-8 error: {e}"),
            LuksError::LuksHeaderError(e) => write!(f, "Luks Header Error: {e}"),
            LuksError::UnknownFilesystem(e) => write!(f, "Not an LUKS superblock: {e}"),
            LuksError::NixError(e) => write!(f, "*Nix operation failed: {e}"),
        }
    }
}

impl From<LuksError> for ContError {
    fn from(err: LuksError) -> Self {
        match err {
            LuksError::IoError(e) => ContError::IoError(e),
            LuksError::UuidConversionError(_) => ContError::InvalidHeader("Invalid string to convert to uuid"),
            LuksError::UTF8ErrorError(_) => ContError::InvalidHeader("Invalid utf8 to convert to string"),
            LuksError::LuksHeaderError(info) => ContError::InvalidHeader(info),
            LuksError::UnknownFilesystem(info) => ContError::UnknownContainer(info),
            LuksError::NixError(e) => ContError::NixError(e),
        }
    }
}

impl From<IoError> for LuksError {
    fn from(err: IoError) -> Self {
        LuksError::IoError(err)
    }
}

impl From<uuid::Error> for LuksError {
    fn from(err: uuid::Error) -> Self {
        LuksError::UuidConversionError(err)
    }
}

impl From<Utf8Error> for LuksError {
    fn from(err: Utf8Error) -> Self {
        LuksError::UTF8ErrorError(err)
    }
}

impl From<rustix::io::Errno> for LuksError {
    fn from(err: rustix::io::Errno) -> Self {
        LuksError::NixError(err)
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
    magics: Some(&[
        BlockidMagic {
            magic: &LUKS1_MAGIC,
            len: 6,
            b_offset: 0,
        },
    ])
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
    magics: Some(&[
        BlockidMagic {
            magic: &LUKS2_MAGIC,
            len: 6,
            b_offset: 0,
        },
    ])
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
    magics: None
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
    fn get_uuid(
            self
        ) -> Result<Uuid, LuksError>
    {
        // This is janky
        let uuid_str = str::from_utf8(&self.uuid)?;
        let uuid = Uuid::from_str(&uuid_str.trim_end_matches('\0'))?;

        return Ok(uuid);
    }

    fn luks_valid(
            self,
        ) -> bool
    {
        if self.magic == LUKS1_MAGIC &&
            u16::from(self.version) == 1
        {
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
        if self.magic == LUKS1_MAGIC && u16::from(self.version) == 2 {
            return true;
        }
        
        for offset in SECONDARY_OFFSETS {
            match from_file::<Luks2Header, R>(file, offset) {
                Ok(secondary) => {
                    if u16::from(secondary.version) == 2 && 
                        u64::from(secondary.hdr_offset) == offset 
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
    let header: Luks1Header = from_file(&mut probe.file, probe.offset)?;
    
    if !header.luks_valid() {
        return Err(LuksError::LuksHeaderError("Luks is not valid luks1 container"));
    }

    probe.push_result(ProbeResult::Container(
                ContainerResults { 
                    cont_type: Some(crate::ContType::LUKS1), 
                    label: None, 
                    cont_uuid: Some(BlockidUUID::Uuid(header.get_uuid()?)), 
                    cont_creator: None, 
                    usage: Some(UsageType::Crypto), 
                    version: Some(BlockidVersion::Number(u64::from(header.version))), 
                    sbmagic: Some(&LUKS1_MAGIC), 
                    sbmagic_offset: Some(0), 
                    cont_size: None, 
                    cont_block_size: None,
                    endianness: Some(Endianness::Big),
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
    let header: Luks2Header = from_file(&mut probe.file, probe.offset)?;

    if !header.luks_valid(&mut probe.file) {
        return Err(LuksError::LuksHeaderError("Luks is not valid luks2 container"));
    }

    probe.push_result(ProbeResult::Container(
                ContainerResults { 
                    cont_type: Some(crate::ContType::LUKS1), 
                    label: None, 
                    cont_uuid: Some(BlockidUUID::Uuid(header.get_uuid()?)), 
                    cont_creator: None, 
                    usage: Some(UsageType::Crypto), 
                    version: Some(BlockidVersion::Number(u64::from(header.version))), 
                    sbmagic: Some(&LUKS2_MAGIC), 
                    sbmagic_offset: Some(0), 
                    cont_size: None, 
                    cont_block_size: None,
                    endianness: Some(Endianness::Big),
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
    let header: Luks2Header = from_file(&mut probe.file, probe.offset)?;

    if !header.luks_valid(&mut probe.file) {
        return Err(LuksError::LuksHeaderError("Luks is not valid luks2 opal container"));
    }

    if header.subsystem[0..7] == LUKS2_HW_OPAL_SUBSYSTEM {
        return Err(LuksError::LuksHeaderError("Luks2 does not contain opal subsystem to be opal"));
    }

    if probe.is_opal_locked()? {
        return Err(LuksError::IoError(ErrorKind::PermissionDenied.into()));
    }

    probe.push_result(ProbeResult::Container(
                ContainerResults { 
                    cont_type: Some(crate::ContType::LUKS1), 
                    label: None, 
                    cont_uuid: Some(BlockidUUID::Uuid(header.get_uuid()?)), 
                    cont_creator: None, 
                    usage: Some(UsageType::Crypto), 
                    version: Some(BlockidVersion::Number(u64::from(header.version))), 
                    sbmagic: Some(&LUKS2_MAGIC), 
                    sbmagic_offset: Some(0), 
                    cont_size: None, 
                    cont_block_size: None,
                    endianness: Some(Endianness::Big),
                }
            )
        );

    return Ok(());
}