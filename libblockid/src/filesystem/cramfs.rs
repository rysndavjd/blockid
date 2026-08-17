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

#[derive(Debug, Clone)]
pub enum CramfsError {
    HeaderChecksumInvalid,
}

impl fmt::Display for CramfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl<E: fmt::Debug> From<CramfsError> for Error<E> {
    fn from(e: CramfsError) -> Self {
        Self::Cramfs(e)
    }
}

const LITTLE_ENDIAN_MAGIC: &[u8; 4] = b"\x45\x3d\xcd\x28";
const BIG_ENDIAN_MAGIC: &[u8; 4] = b"\x28\xcd\x3d\x45";

pub const CRAMFS_MINSZ: Option<u64> = None;
pub const CRAMFS_MAGICS: Option<&'static [Magic]> = Some(&[
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
    crc: [u8; 2],
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
        u16::from_le_bytes(sb.crc)
    } else {
        u16::from_be_bytes(sb.crc)
    };

    let csummed_size = if le {
        u32::from_le_bytes(sb.size)
    } else {
        u32::from_be_bytes(sb.size)
    };

    if csummed_size > (1 << 16) || csummed_size < (size_of::<CramfsSuperBlock>() as u32) {
        todo!()
    }

    let crc_buf = reader.read_at_exclude(
        offset,
        csummed_size as usize,
        offset_of!(CramfsSuperBlock, crc)..(offset_of!(CramfsSuperBlock, crc) + 2),
    )?;

    let calc_sum = Crc::<u32>::new(&CRC_32_ISO_HDLC).checksum(&crc_buf);

    if calc_sum == expected.into() {
        return Ok(());
    }

    Err(CramfsError::HeaderChecksumInvalid.into())
}

pub fn probe_cramfs<IO: BlockIo>(
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

    if v2 && verify_csum(reader, offset, sb, le).is_err() {
        todo!()
    }

    let mut info = FsInfo::empty();

    info.set_fs_type(FsType::Cramfs);
    if sb.name != [0u8; 16] {
        info.set_label(BString::from(sb.name).into());
    }
    info.set_version(if v2 { "2".to_string() } else { "1".to_string() });
    info.set_magic(magic.bytes.to_vec(), magic.offset);

    if le {
        info.set_fs_size(u32::from_le_bytes(sb.size).into());
        info.set_endianness(Endianness::Little);
    } else {
        info.set_fs_size(u32::from_be_bytes(sb.size).into());
        info.set_endianness(Endianness::Big);
    }

    return Ok(info);
}
