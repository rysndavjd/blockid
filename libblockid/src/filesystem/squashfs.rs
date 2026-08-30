use zerocopy::{FromBytes, Immutable, IntoBytes, Unaligned, transmute_ref};

use crate::{
    Endianness,
    error::Error,
    filesystem::{FsInfo, FsType},
    io::{BlockIo, Reader},
    probe::Magic,
    std::fmt,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SquashfsError {
    InvalidVersion,
    InvalidVersionSqsh3,
}

impl fmt::Display for SquashfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SquashfsError::InvalidVersion => write!(f, "invalid squashfs version"),
            SquashfsError::InvalidVersionSqsh3 => write!(f, "invalid squashfs3 version"),
        }
    }
}

impl<E: fmt::Debug> From<SquashfsError> for Error<E> {
    fn from(e: SquashfsError) -> Self {
        Error::Squashfs(e)
    }
}

const MAGIC_LITTLE: [u8; 4] = *b"hsqs";
const MAGIC_BIG: [u8; 4] = *b"sqsh";

pub const SQUASHFS_MINSZ: Option<u64> = None;
pub const SQUASHFS_MAGICS: Option<&'static [Magic]> = Some(&[Magic {
    bytes: &MAGIC_LITTLE,
    offset: 0,
}]);

pub const SQUASHFS3_MAGICS: Option<&'static [Magic]> = Some(&[
    Magic {
        bytes: &MAGIC_BIG,
        offset: 0,
    },
    Magic {
        bytes: &MAGIC_LITTLE,
        offset: 0,
    },
]);

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct SquashfsHeader {
    magic: [u8; 4],
    inode_count: [u8; 4],
    mod_time: [u8; 4],
    block_size: [u8; 4],
    frag_count: [u8; 4],
    compressor: [u8; 2],
    block_log: [u8; 2],
    flags: [u8; 2],
    id_count: [u8; 2],
    version_major: [u8; 2],
    version_minor: [u8; 2],
    root_inode: [u8; 8],
    bytes_used: [u8; 8],
    id_table: [u8; 8],
    xattr_table: [u8; 8],
    inode_table: [u8; 8],
    dir_table: [u8; 8],
    frag_table: [u8; 8],
    export_table: [u8; 8],
}

pub fn probe_squashfs<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    _: Magic,
) -> Result<FsInfo, Error<IO::Error>> {
    let buf: [u8; size_of::<SquashfsHeader>()] = reader.read_exact_at(offset)?;
    let sb: &SquashfsHeader = transmute_ref!(&buf);

    let vermaj = u16::from_le_bytes(sb.version_major);
    let vermin = u16::from_le_bytes(sb.version_minor);

    if vermaj < 4 {
        return Err(SquashfsError::InvalidVersion.into());
    }

    let mut info = FsInfo::empty();

    info.set_fs_type(FsType::Squashfs);
    info.set_version(format!("{}.{}", vermaj, vermin));
    info.set_magic(sb.magic.to_vec(), offset);
    info.set_fs_block_size(u32::from_le_bytes(sb.block_size).into());
    info.set_block_size(u32::from_le_bytes(sb.block_size).into());
    info.set_fs_size(u64::from_le_bytes(sb.bytes_used));

    return Ok(info);
}

pub fn probe_squashfs3<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    magic: Magic,
) -> Result<FsInfo, Error<IO::Error>> {
    let buf: [u8; size_of::<SquashfsHeader>()] = reader.read_exact_at(offset)?;
    let sb: &SquashfsHeader = transmute_ref!(&buf);

    let (vermaj, vermin, endianness) = if magic.bytes == MAGIC_LITTLE {
        (
            u16::from_le_bytes(sb.version_major),
            u16::from_le_bytes(sb.version_minor),
            Endianness::Little,
        )
    } else {
        (
            u16::from_be_bytes(sb.version_major),
            u16::from_be_bytes(sb.version_minor),
            Endianness::Big,
        )
    };

    if vermaj > 3 {
        return Err(SquashfsError::InvalidVersionSqsh3.into());
    }

    let mut info = FsInfo::empty();

    info.set_fs_type(FsType::Squashfs3);
    info.set_version(format!("{}.{}", vermaj, vermin));
    info.set_magic(sb.magic.to_vec(), offset);
    info.set_fs_block_size(1024);
    info.set_block_size(1024);
    info.set_endianness(endianness);

    return Ok(info);
}
