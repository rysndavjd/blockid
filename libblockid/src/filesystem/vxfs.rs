use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, transmute_ref};

use crate::{
    BlockTag,
    error::Error,
    filesystem::BlockInfo,
    io::{BlockIo, Reader},
    probe::{Endianness, Magic, ProbeFlags},
    std::fmt,
};

#[derive(Debug, Clone)]
pub enum VxfsError {}

// impl fmt::Display for VxfsError {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {}
//     }
// }

impl<E: fmt::Debug> From<VxfsError> for Error<E> {
    fn from(e: VxfsError) -> Self {
        Error::Vxfs(e)
    }
}

pub const VXFS_MINSZ: Option<u64> = None;
pub const VXFS_MAGICS: Option<&'static [Magic]> = Some(&[
    Magic {
        magic: LITTLE_ENDIAN_MAGIC,
        len: 4,
        b_offset: 1024,
    },
    Magic {
        magic: BIG_ENDIAN_MAGIC,
        len: 4,
        b_offset: 8192,
    },
]);

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout)]
struct VxfsSuperBlock {
    vs_magic: [u8; 4],
    vs_version: [u8; 4],
    vs_ctime: [u8; 4],
    vs_cutime: [u8; 4],
    unused1: [u8; 4],
    unused2: [u8; 4],
    vs_old_logstart: [u8; 4],
    vs_old_logend: [u8; 4],
    vs_bsize: [u8; 4],
    vs_size: [u8; 4],
    vs_dsize: [u8; 4],
}

const LITTLE_ENDIAN_MAGIC: &[u8; 4] = b"\xf5\xfc\x01\xa5";
const BIG_ENDIAN_MAGIC: &[u8; 4] = b"\xa5\x01\xfc\xf5";

pub fn probe_vxfs<IO: BlockIo>(
    reader: &mut Reader<IO>,
    _: ProbeFlags,
    offset: u64,
    magic: Magic,
) -> Result<BlockInfo, Error<IO::Error>> {
    let buf: [u8; size_of::<VxfsSuperBlock>()] = reader.read_exact_at(offset)?;

    let xvfs: &VxfsSuperBlock = transmute_ref!(&buf);

    let mut info = BlockInfo::new();

    if magic.magic == LITTLE_ENDIAN_MAGIC {
        info.set(BlockTag::Version(format!(
            "{}",
            u32::from_le_bytes(xvfs.vs_version)
        )));
        info.set(BlockTag::FsBlockSize(
            u32::from_le_bytes(xvfs.vs_bsize).into(),
        ));
        info.set(BlockTag::BlockSize(
            u32::from_le_bytes(xvfs.vs_bsize).into(),
        ));
        info.set(BlockTag::Endianness(Endianness::Little));
    } else {
        info.set(BlockTag::Version(format!(
            "{}",
            u32::from_be_bytes(xvfs.vs_version)
        )));
        info.set(BlockTag::FsBlockSize(
            u32::from_be_bytes(xvfs.vs_bsize).into(),
        ));
        info.set(BlockTag::BlockSize(
            u32::from_be_bytes(xvfs.vs_bsize).into(),
        ));
        info.set(BlockTag::Endianness(Endianness::Big));
    };

    return Ok(info);
}
