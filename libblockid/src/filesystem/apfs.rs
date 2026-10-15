use uuid::Uuid;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, Unaligned,
    byteorder::{LittleEndian, U16, U32, U64},
    transmute_ref,
};

use crate::{
    error::Error,
    filesystem::{FsId, FsInfo, FsType},
    io::{BlockIo, Reader},
    probe::Magic,
    std::{fmt, mem::offset_of},
};

/// Errors that may occur when parsing APFS (Apple File System).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ApfsError {
    /// Stored header checksum does not match checksum calculated from
    /// header contents.
    HeaderChecksumInvalid,
    /// Invalid APFS container superblock filesystem type.
    InvalidSuperFsType,
    /// Invalid APFS container superblock subtype.
    InvalidSuperblockSubType,
    /// Padding within header not zeroed.
    PaddingNotZero,
    /// Block size is not 4096 bytes.
    InvalidBlockSize,
    /// Header UUID is [nil](https://en.wikipedia.org/wiki/Universally_unique_identifier#Special_values).
    UuidNil,
}

impl fmt::Display for ApfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApfsError::HeaderChecksumInvalid => write!(f, "invalid header checksum"),
            ApfsError::InvalidSuperFsType => {
                write!(f, "invalid APFS container superblock filesystem type")
            }
            ApfsError::InvalidSuperblockSubType => {
                write!(f, "invalid APFS container superblock subtype")
            }
            ApfsError::PaddingNotZero => write!(f, "padding within header not zeroed"),
            ApfsError::InvalidBlockSize => write!(f, "block size is not 4096 bytes"),
            ApfsError::UuidNil => write!(f, "UUID is nil"),
        }
    }
}

impl<E: fmt::Debug> From<ApfsError> for Error<E> {
    fn from(e: ApfsError) -> Self {
        Self::Apfs(e)
    }
}

pub(crate) const APFS_MINSZ: Option<u64> = None;
pub(crate) const APFS_MAGICS: Option<&'static [Magic]> = Some(&[Magic {
    bytes: ApfsSuperBlock::MAGIC,
    offset: ApfsSuperBlock::MAGIC_OFFSET,
}]);

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
struct ApfsSuperBlock {
    checksum: U64<LittleEndian>,
    oid: U64<LittleEndian>,
    xid: U64<LittleEndian>,
    apfs_type: U16<LittleEndian>,
    flags: U16<LittleEndian>,
    subtype: U16<LittleEndian>,
    pad: U16<LittleEndian>,

    magic: [u8; 4],
    block_size: U32<LittleEndian>,
    block_count: U64<LittleEndian>,
    features: U64<LittleEndian>,
    read_only_features: U64<LittleEndian>,
    incompatible_features: U64<LittleEndian>,
    uuid: [u8; 16],
    padding: [u8; 4008],
}

impl ApfsSuperBlock {
    const MAGIC: &[u8; 4] = b"NXSB";
    const MAGIC_OFFSET: u64 = 32;

    const CONTAINER_SUPERBLOCK_TYPE: u16 = 1;
    const CONTAINER_SUPERBLOCK_SUBTYPE: u16 = 0;
    const STANDARD_BLOCK_SIZE: u32 = 4096;
}

fn fletcher64(buf: &[u8]) -> u64 {
    let mut lo32: u64 = 0;
    let mut hi32: u64 = 0;

    for i in 0..(buf.len() / 4) {
        let offset = i * 4;
        let word = u32::from_le_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]) as u64;
        lo32 = lo32.wrapping_add(word);
        hi32 = hi32.wrapping_add(lo32);
    }

    let csum_lo = !((lo32.wrapping_add(hi32)) % 0xFFFFFFFF) as u32;
    let csum_hi = !((lo32.wrapping_add(csum_lo as u64)) % 0xFFFFFFFF) as u32;

    return ((csum_hi as u64) << 32) | (csum_lo as u64);
}

pub(crate) fn probe_apfs<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    magic: Magic,
) -> Result<FsInfo, Error<IO::Error>> {
    let buf: [u8; size_of::<ApfsSuperBlock>()] = reader.read_exact_at(offset)?;
    let sb: &ApfsSuperBlock = transmute_ref!(&buf);

    let csum = fletcher64(&buf[offset_of!(ApfsSuperBlock, oid)..]);

    if sb.checksum.get() != csum {
        return Err(ApfsError::HeaderChecksumInvalid.into());
    }

    if sb.apfs_type.get() != ApfsSuperBlock::CONTAINER_SUPERBLOCK_TYPE {
        return Err(ApfsError::InvalidSuperFsType.into());
    }

    if sb.subtype.get() != ApfsSuperBlock::CONTAINER_SUPERBLOCK_SUBTYPE {
        return Err(ApfsError::InvalidSuperblockSubType.into());
    }

    if sb.pad.get() != 0 {
        return Err(ApfsError::PaddingNotZero.into());
    }

    if sb.block_size.get() != ApfsSuperBlock::STANDARD_BLOCK_SIZE {
        return Err(ApfsError::InvalidBlockSize.into());
    }

    let uuid = if sb.uuid != [0u8; 16] {
        Uuid::from_bytes(sb.uuid)
    } else {
        return Err(ApfsError::UuidNil.into());
    };

    let mut info = FsInfo::empty();

    info.set_fs_type(FsType::Apfs);
    info.set_fs_id(FsId::Uuid(uuid));
    info.set_fs_block_size(u64::from(sb.block_size));
    info.set_block_size(u64::from(sb.block_size));
    info.set_magic(magic.bytes, magic.offset);

    return Ok(info);
}
