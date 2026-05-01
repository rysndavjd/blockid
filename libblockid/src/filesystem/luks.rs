use uuid::Uuid;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, Unaligned,
    byteorder::{BigEndian, U16, U32, U64},
    transmute_ref,
};

use crate::{
    BlockInfo, Id,
    error::Error,
    filesystem::{BlockTag, BlockType},
    io::{BlockIo, Reader},
    probe::{Magic, Usage},
    std::{fmt, str::FromStr},
    util::{UtfError, decode_utf8_from},
};

/*
 * https://en.wikipedia.org/wiki/Linux_Unified_Key_Setup#LUKS2
 * https://cdn.kernel.org/pub/linux/utils/cryptsetup/LUKS_docs/on-disk-format.pdf
 * https://gitlab.com/cryptsetup/LUKS2-docs
*/

#[derive(Debug)]
pub enum LuksError {
    UuidConversionError(uuid::Error),
    UtfError(UtfError),
    InvalidLuks1,
    InvalidLuks2,
    InvalidLuks2Opal,
}

impl fmt::Display for LuksError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LuksError::UuidConversionError(e) => write!(f, "UUID conversion faild: {e}"),
            LuksError::UtfError(e) => write!(f, "UTF operation failed: {e}"),
            LuksError::InvalidLuks1 => write!(f, "Invalid LUKS1 header"),
            LuksError::InvalidLuks2 => write!(f, "Invalid LUKS2 header"),
            LuksError::InvalidLuks2Opal => write!(f, "Invalid LUKS2 Opal header"),
        }
    }
}

impl From<uuid::Error> for LuksError {
    fn from(e: uuid::Error) -> Self {
        Self::UuidConversionError(e)
    }
}

impl From<UtfError> for LuksError {
    fn from(e: UtfError) -> Self {
        Self::UtfError(e)
    }
}

impl<E: core::fmt::Debug> From<LuksError> for Error<E> {
    fn from(e: LuksError) -> Self {
        Error::Luks(e)
    }
}

pub const LUKS1_MAGIC: [u8; 6] = *b"LUKS\xba\xbe";
pub const LUKS2_MAGIC: [u8; 6] = *b"SKUL\xba\xbe";
pub const LUKS2_HW_OPAL_SUBSYSTEM: [u8; 7] = *b"HW-OPAL";

pub const SECONDARY_OFFSETS: [u64; 9] = [
    0x04000, 0x008000, 0x010000, 0x020000, 0x40000, 0x080000, 0x100000, 0x200000, 0x400000,
];

pub const LUKS1_MAGICS: Option<&'static [Magic]> = Some(&[Magic {
    magic: &LUKS1_MAGIC,
    len: 6,
    b_offset: 0,
}]);

pub const LUKS2_MAGICS: Option<&'static [Magic]> = Some(&[Magic {
    magic: &LUKS2_MAGIC,
    len: 6,
    b_offset: 0,
}]);

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
    fn luks_valid<IO: BlockIo>(self, reader: &mut Reader<IO>) -> Result<bool, Error<IO::Error>> {
        if self.magic == LUKS1_MAGIC && u16::from(self.version) == 2 {
            return Ok(true);
        }

        let mut buf: [u8; size_of::<Luks2Header>()] = [0u8; size_of::<Luks2Header>()];
        for offset in SECONDARY_OFFSETS {
            reader.read_at(offset, &mut buf).map_err(Error::Io)?;

            let hdr: &Luks2Header = transmute_ref!(&buf);

            if u16::from(hdr.version) == 2 && u64::from(hdr.hdr_offset) == offset {
                return Ok(true);
            }
        }

        return Ok(false);
    }
}

pub fn probe_luks1<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    magic: Magic,
) -> Result<BlockInfo, Error<IO::Error>> {
    let buf: [u8; size_of::<Luks1Header>()] = reader.read_exact_at(offset).map_err(Error::Io)?;

    let sb: &Luks1Header = transmute_ref!(&buf);

    if !sb.luks_valid() {
        return Err(LuksError::InvalidLuks1.into());
    }

    let utf = decode_utf8_from(&sb.uuid).map_err(LuksError::UtfError)?;
    let uuid = Uuid::from_str(&utf).map_err(LuksError::UuidConversionError)?;

    let version = sb.version.to_string();

    let mut info = BlockInfo::new();

    info.set(BlockTag::BlockType(BlockType::LUKS1));
    info.set(BlockTag::Id(Id::Uuid(uuid)));
    info.set(BlockTag::Usage(Usage::Crypto));
    info.set(BlockTag::Version(version));
    info.set(BlockTag::Magic(magic.magic.to_vec()));
    info.set(BlockTag::MagicOffset(magic.b_offset));

    return Ok(info);
}

pub fn probe_luks2<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    magic: Magic,
) -> Result<BlockInfo, Error<IO::Error>> {
    let buf: [u8; size_of::<Luks2Header>()] = reader.read_exact_at(offset).map_err(Error::Io)?;

    let sb: &Luks2Header = transmute_ref!(&buf);

    if !sb.luks_valid(reader)? {
        return Err(LuksError::InvalidLuks2.into());
    }

    let utf = decode_utf8_from(&sb.uuid).map_err(LuksError::UtfError)?;
    let uuid = Uuid::from_str(&utf).map_err(LuksError::UuidConversionError)?;

    let version = sb.version.to_string();

    let mut info = BlockInfo::new();

    info.set(BlockTag::BlockType(BlockType::LUKS2));
    info.set(BlockTag::Id(Id::Uuid(uuid)));
    info.set(BlockTag::Usage(Usage::Crypto));
    info.set(BlockTag::Version(version));
    info.set(BlockTag::Magic(magic.magic.to_vec()));
    info.set(BlockTag::MagicOffset(magic.b_offset));

    return Ok(info);
}

pub fn probe_luks_opal<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    magic: Magic,
) -> Result<BlockInfo, Error<IO::Error>> {
    let buf: [u8; size_of::<Luks2Header>()] = reader.read_exact_at(offset).map_err(Error::Io)?;

    let sb: &Luks2Header = transmute_ref!(&buf);

    if !sb.luks_valid(reader)? {
        return Err(LuksError::InvalidLuks2Opal.into());
    }

    if sb.subsystem[0..7] != LUKS2_HW_OPAL_SUBSYSTEM {
        return Err(LuksError::InvalidLuks2Opal.into());
    }

    let utf = decode_utf8_from(&sb.uuid).map_err(LuksError::UtfError)?;
    let uuid = Uuid::from_str(&utf).map_err(LuksError::UuidConversionError)?;

    let version = sb.version.to_string();

    let mut info = BlockInfo::new();

    info.set(BlockTag::BlockType(BlockType::LUKSOpal));
    info.set(BlockTag::Id(Id::Uuid(uuid)));
    info.set(BlockTag::Usage(Usage::Crypto));
    info.set(BlockTag::Version(version));
    info.set(BlockTag::Magic(magic.magic.to_vec()));
    info.set(BlockTag::MagicOffset(magic.b_offset));

    return Ok(info);
}
