use bstr::BString;
use crc::{CRC_32_ISO_HDLC, Crc};
use zerocopy::{FromBytes, Immutable, IntoBytes, Unaligned, transmute_ref};

use crate::{
    Endianness,
    error::Error,
    filesystem::{FsInfo, FsType},
    io::{BlockIo, Reader},
    probe::Magic,
    std::{fmt, mem::offset_of},
};

/// Errors that may occur when parsing cramfs.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum CramfsError {
    /// Version 2 header contains invalid checksum.
    HeaderChecksumInvalid,
    /// Header size less than 64 or greater than 65536 bytes.
    HeaderSizeInvalid,
}

impl fmt::Display for CramfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CramfsError::HeaderChecksumInvalid => write!(f, "invalid v2 header checksum"),
            CramfsError::HeaderSizeInvalid => {
                write!(f, "header size less than 64 or greater than 65536 bytes")
            }
        }
    }
}

impl<E: fmt::Debug> From<CramfsError> for Error<E> {
    fn from(e: CramfsError) -> Self {
        Self::Cramfs(e)
    }
}

const LITTLE_ENDIAN_MAGIC: &[u8; 4] = b"\x45\x3d\xcd\x28";
const BIG_ENDIAN_MAGIC: &[u8; 4] = b"\x28\xcd\x3d\x45";

pub(crate) const CRAMFS_MINSZ: Option<u64> = None;
pub(crate) const CRAMFS_MAGICS: Option<&'static [Magic]> = Some(&[
    Magic {
        bytes: LITTLE_ENDIAN_MAGIC,
        offset: 0,
    },
    Magic {
        bytes: BIG_ENDIAN_MAGIC,
        offset: 0,
    },
]);

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
struct CramfsSuperBlock {
    magic: [u8; 4],
    size: [u8; 4],
    flags: [u8; 4],
    future: [u8; 4],
    signature: [u8; 16],
    crc: [u8; 4],
    edition: [u8; 4],
    blocks: [u8; 4],
    files: [u8; 4],
    name: [u8; 16],
}

impl CramfsSuperBlock {
    const FLAG_FSID_VERSION_2: u32 = 0x00000001;
}

fn verify_csum<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    sb: &CramfsSuperBlock,
    le: bool,
) -> Result<(), Error<IO::Error>> {
    let expected = if le {
        u32::from_le_bytes(sb.crc)
    } else {
        u32::from_be_bytes(sb.crc)
    };

    let sb_size = if le {
        u32::from_le_bytes(sb.size)
    } else {
        u32::from_be_bytes(sb.size)
    };

    if sb_size > (1 << 16) || sb_size < (size_of::<CramfsSuperBlock>() as u32) {
        return Err(CramfsError::HeaderSizeInvalid.into());
    }

    let crc_buf = reader.read_at_exclude(
        offset,
        sb_size as usize,
        offset_of!(CramfsSuperBlock, crc)
            ..(offset_of!(CramfsSuperBlock, crc) + size_of_val(&sb.crc)),
    )?;

    let calc_sum = Crc::<u32>::new(&CRC_32_ISO_HDLC).checksum(&crc_buf);

    if calc_sum == expected {
        return Ok(());
    }

    Err(CramfsError::HeaderChecksumInvalid.into())
}

pub(crate) fn probe_cramfs<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    magic: Magic,
) -> Result<FsInfo, Error<IO::Error>> {
    let buf: [u8; size_of::<CramfsSuperBlock>()] = reader.read_exact_at(offset)?;
    let sb: &CramfsSuperBlock = transmute_ref!(&buf);

    let le = magic.bytes == LITTLE_ENDIAN_MAGIC;
    let v2 = (if le {
        u32::from_le_bytes(sb.flags)
    } else {
        u32::from_be_bytes(sb.flags)
    }) & CramfsSuperBlock::FLAG_FSID_VERSION_2
        != 0;

    if v2 {
        verify_csum(reader, offset, sb, le)?;
    }

    let mut info = FsInfo::empty();

    info.set_fs_type(FsType::Cramfs);
    if sb.name != [0u8; 16] {
        info.set_label(BString::from(sb.name).into());
    }
    info.set_version(if v2 { "2".to_string() } else { "1".to_string() });
    info.set_magic(magic.bytes, magic.offset);

    if le {
        info.set_fs_size(u32::from_le_bytes(sb.size).into());
        info.set_endianness(Endianness::Little);
    } else {
        info.set_fs_size(u32::from_be_bytes(sb.size).into());
        info.set_endianness(Endianness::Big);
    }

    return Ok(info);
}
