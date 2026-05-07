use std::mem::offset_of;

use uuid::Uuid;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, Unaligned, byteorder::{LittleEndian, U16, U32, U64}, transmute_ref,
};
use crate::{error::Error, io::{BlockIo, Reader}, probe::{Magic, Id, Usage}, std::fmt, filesystem::{BlockInfo, BlockTag, BlockType}, util::fletcher64};

#[derive(Debug)]
pub enum ApfsError {
    HeaderChecksumInvalid,
    InvalidSuperblockType,
    InvalidSuperblockSubType,
    PaddingNotZero,
    InvalidBlockSize,
    UuidEmpty,
}

impl fmt::Display for ApfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApfsError::HeaderChecksumInvalid => write!(f, "Invalid header checksum"),
            ApfsError::InvalidSuperblockType => write!(f, "Invalid APFS container superblock type"),
            ApfsError::InvalidSuperblockSubType => write!(f, "Invalid APFS container superblock subtype"),
            ApfsError::PaddingNotZero => write!(f, "Padding not zero"),
            ApfsError::InvalidBlockSize => write!(f, "Invalid standard block size"),
            ApfsError::UuidEmpty => write!(f, "UUID entry is empty"),
        }
    }
}

impl<E: fmt::Debug> From<ApfsError> for Error<E> {
    fn from(e: ApfsError) -> Self {
        Self::Apfs(e)
    }
}

pub const APFS_MINSZ: Option<u64> = None;
pub const APFS_MAGICS: Option<&'static [Magic]> = Some(&[Magic {
    magic: ApfsSuperBlock::MAGIC,
    len: ApfsSuperBlock::MAGIC.len(),
    b_offset: ApfsSuperBlock::MAGIC_OFFSET,
}]);

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct ApfsSuperBlock {
    pub checksum: U64<LittleEndian>,
    pub oid: U64<LittleEndian>,
    pub xid: U64<LittleEndian>,
    pub apfs_type: U16<LittleEndian>,
    pub flags: U16<LittleEndian>,
    pub subtype: U16<LittleEndian>,
    pub pad: U16<LittleEndian>,

    pub magic: [u8; 4],
    pub block_size: U32<LittleEndian>,
    pub block_count: U64<LittleEndian>,
    pub features: U64<LittleEndian>,
    pub read_only_features: U64<LittleEndian>,
    pub incompatible_features: U64<LittleEndian>,
    pub uuid: [u8; 16],
    pub padding: [u8; 4008],
}

impl ApfsSuperBlock {
    const MAGIC: &[u8; 4] = b"NXSB";
    const MAGIC_OFFSET: u64 = 32;

    const CONTAINER_SUPERBLOCK_TYPE: u16 = 1;
    const CONTAINER_SUPERBLOCK_SUBTYPE: u16 = 0;
    const STANDARD_BLOCK_SIZE: u32 = 4096;

}

pub fn probe_apfs<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    _: Magic,
) -> Result<BlockInfo, Error<IO::Error>> {
    let buf: [u8; size_of::<ApfsSuperBlock>()] = reader.read_exact_at(offset)?;
    let sb: &ApfsSuperBlock = transmute_ref!(&buf);

    let csum = fletcher64(&buf[offset_of!(ApfsSuperBlock, oid)..]);

    if u64::from(sb.checksum) != csum {
        return Err(ApfsError::HeaderChecksumInvalid.into());
    }

    if u16::from(sb.apfs_type) != ApfsSuperBlock::CONTAINER_SUPERBLOCK_TYPE {
        return Err(ApfsError::InvalidSuperblockType.into());
    }

    if u16::from(sb.subtype) != ApfsSuperBlock::CONTAINER_SUPERBLOCK_SUBTYPE {
        return Err(ApfsError::InvalidSuperblockSubType.into());
    }

    if u16::from(sb.pad) != 0 {
        return Err(ApfsError::PaddingNotZero.into());
    }

    if u32::from(sb.block_size) != ApfsSuperBlock::STANDARD_BLOCK_SIZE {
        return Err(ApfsError::InvalidBlockSize.into());
    }

    let uuid = if sb.uuid != [0u8; 16] {
        Uuid::from_bytes(sb.uuid)
    } else {
        return Err(ApfsError::UuidEmpty.into());
    };

    let mut info = BlockInfo::new();

    info.set(BlockTag::BlockType(BlockType::Apfs));
    info.set(BlockTag::Id(Id::Uuid(uuid)));
    info.set(BlockTag::Usage(Usage::Filesystem));
    info.set(BlockTag::FsBlockSize(u64::from(sb.block_size)));
    info.set(BlockTag::BlockSize(u64::from(sb.block_size)));
    info.set(BlockTag::Usage(Usage::Filesystem));
    info.set(BlockTag::Magic(ApfsSuperBlock::MAGIC.to_vec()));
    info.set(BlockTag::MagicOffset(ApfsSuperBlock::MAGIC_OFFSET));

    return Ok(info);
}