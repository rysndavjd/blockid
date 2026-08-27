use bstr::BString;
use uuid::{Uuid, fmt::Hyphenated};
use zerocopy::{
    FromBytes, Immutable, IntoBytes, Unaligned,
    byteorder::{BigEndian, U16, U32, U64},
    transmute_ref,
};

use crate::{
    error::Error,
    filesystem::{FsInfo, FsType},
    io::{BlockIo, Reader},
    probe::Magic,
    std::fmt,
};

/*
 * https://en.wikipedia.org/wiki/Linux_Unified_Key_Setup#LUKS2
 * https://cdn.kernel.org/pub/linux/utils/cryptsetup/LUKS_docs/on-disk-format.pdf
 * https://gitlab.com/cryptsetup/LUKS2-docs
*/

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LuksError {
    InvalidUuid(uuid::Error),
    UnableLocateHeader,
    InvalidLuks2Opal,
    InvalidVersion,
}

impl fmt::Display for LuksError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LuksError::InvalidUuid(e) => write!(f, "UUID parsing faild: {e}"),
            LuksError::UnableLocateHeader => write!(f, "Unable to locate valid LUKS header"),
            LuksError::InvalidLuks2Opal => write!(f, "Invalid LUKS2 Opal header"),
            LuksError::InvalidVersion => write!(f, "Invalid LUKS version"),
        }
    }
}

impl From<uuid::Error> for LuksError {
    fn from(e: uuid::Error) -> Self {
        Self::InvalidUuid(e)
    }
}

impl<E: fmt::Debug> From<LuksError> for Error<E> {
    fn from(e: LuksError) -> Self {
        Error::Luks(e)
    }
}

const LUKS1_MAGIC: [u8; 6] = *b"LUKS\xba\xbe";
const LUKS2_MAGIC: [u8; 6] = *b"SKUL\xba\xbe";
const LUKS2_HW_OPAL_SUBSYSTEM: [u8; 7] = *b"HW-OPAL";

const SECONDARY_OFFSETS: [u64; 9] = [
    0x04000, 0x008000, 0x010000, 0x020000, 0x40000, 0x080000, 0x100000, 0x200000, 0x400000,
];

pub const LUKS_MAGICS: Option<&'static [Magic]> = None;

pub const LUKS1_MINSZ: Option<u64> = Some(1048576);
pub const LUKS2_MINSZ: Option<u64> = Some(4194304);

pub const LUKSOPAL_MAGICS: Option<&'static [Magic]> = None;

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

fn luks_valid(sb: &Luks2Header, magic: [u8; 6], offset: u64) -> bool {
    if sb.magic == magic {
        return true;
    }

    if u16::from(sb.version) == 2 && u64::from(sb.hdr_offset) != offset {
        return true;
    }

    false
}

fn luks_info(sb: &Luks2Header, offset: u64) -> Result<FsInfo, LuksError> {
    let version = u16::from(sb.version);
    let mut info = FsInfo::empty();

    match version {
        1 => {
            info.set_fs_type(FsType::LUKS1);
        }
        2 => {
            info.set_fs_type(FsType::LUKS2);
            if sb.label != [0u8; 48] {
                info.set_label(BString::from(&sb.label).into());
            }
        }
        _ => return Err(LuksError::InvalidVersion),
    };
    let uuid =
        Uuid::try_parse_ascii(&sb.uuid[..Hyphenated::LENGTH]).map_err(LuksError::InvalidUuid)?;

    info.set_fs_id(uuid.into());
    //todo: use lexical-core
    info.set_version(format!("{}", version));
    info.set_magic(sb.magic.to_vec(), offset);

    return Ok(info);
}

pub fn probe_luks<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    _: Magic,
) -> Result<FsInfo, Error<IO::Error>> {
    let buf: [u8; size_of::<Luks2Header>()] = reader.read_exact_at(offset)?;
    let sb: &Luks2Header = transmute_ref!(&buf);

    if luks_valid(sb, LUKS1_MAGIC, 0) {
        return Ok(luks_info(sb, 0)?);
    }

    for offset in SECONDARY_OFFSETS {
        let buf: [u8; size_of::<Luks2Header>()] = reader.read_exact_at(offset)?;
        let sb: &Luks2Header = transmute_ref!(&buf);

        if luks_valid(sb, LUKS2_MAGIC, offset) {
            return Ok(luks_info(sb, offset)?);
        }
    }

    Err(LuksError::UnableLocateHeader.into())
}

pub fn probe_luks_opal<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    _: Magic,
) -> Result<FsInfo, Error<IO::Error>> {
    // let buf: [u8; size_of::<Luks2Header>()] = reader.read_exact_at(offset)?;

    // let sb: &Luks2Header = transmute_ref!(&buf);

    // if !sb.luks_valid(reader)? {
    //     return Err(LuksError::InvalidLuks2Opal.into());
    // }

    // if sb.subsystem[0..7] != LUKS2_HW_OPAL_SUBSYSTEM {
    //     return Err(LuksError::InvalidLuks2Opal.into());
    // }

    // let uuid = Uuid::try_parse_ascii(&sb.uuid[..uuid::fmt::Hyphenated::LENGTH])
    //     .map_err(LuksError::UuidError)?;

    // let mut info = FsInfo::empty();

    // info.set_fs_type(FsType::LUKS2);
    // info.set_fs_id(uuid.into());
    // info.set_version(format!("{}", sb.version));
    // info.set_magic(magic.bytes.to_vec(), magic.offset);

    // return Ok(info);

    todo!()
}
