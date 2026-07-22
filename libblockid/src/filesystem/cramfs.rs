use std::mem::offset_of;

use crc::{CRC_32_ISO_HDLC, Crc};
use zerocopy::{FromBytes, Immutable, IntoBytes, Unaligned, transmute_ref};

use crate::{
    error::Error,
    filesystem::BlockInfo,
    io::{BlockIo, Reader},
    probe::{Magic, ProbeFlags},
    std::fmt,
};

#[derive(Debug, Clone)]
pub enum CramfsError {}

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
        magic: LITTLE_ENDIAN_MAGIC,
        b_offset: 0,
    },
    Magic {
        magic: BIG_ENDIAN_MAGIC,
        b_offset: 0,
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
    mag: Magic,
    sb: CramfsSuperBlock,
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

    todo!()
}

pub fn probe_cramfs<IO: BlockIo>(
    reader: &mut Reader<IO>,
    flags: ProbeFlags,
    offset: u64,
    mag: Magic,
) -> Result<BlockInfo, Error<IO::Error>> {
    let buf: [u8; size_of::<CramfsSuperBlock>()] = reader.read_exact_at(offset)?;

    let sb: &CramfsSuperBlock = transmute_ref!(&buf);

    let le = mag.magic == LITTLE_ENDIAN_MAGIC;

    let v2 = (if le {
        u32::from_le_bytes(sb.flags)
    } else {
        u32::from_be_bytes(sb.flags)
    }) & CramfsSuperBlock::FLAG_FSID_VERSION_2
        != 0;

    todo!()
}
