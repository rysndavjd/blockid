use std::io;

use bitflags::bitflags;
use zerocopy::{FromBytes, IntoBytes, Unaligned, 
    byteorder::U64, byteorder::U32, byteorder::U16, 
    byteorder::BigEndian, byteorder::LittleEndian, Immutable};
use rustix::fs::makedev;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    read_as, FilesystemResults,
    BlockidError, BlockidIdinfo, BlockidMagic, BlockidProbe,
    BlockidUUID, BlockidVersion, FsType, ProbeResult, UsageType,
    filesystems::FsError,
};

/*
https://www.kernel.org/doc/html/latest/filesystems/ext4/globals.html
*/

#[derive(Error, Debug)]
pub enum SwapError {
    #[error("I/O operation failed: {0}")]
    IoError(#[from] io::Error),
    #[error("Swap header error: {0}")]
    SwapHeaderError(&'static str),
    #[error("Not an Swap superblock: {0}")]
    UnknownFilesystem(&'static str),
}

impl From<SwapError> for FsError {
    fn from(err: SwapError) -> Self {
        match err {
            SwapError::IoError(e) => FsError::IoError(e),
            SwapError::SwapHeaderError(e) => FsError::InvalidHeader(e),
            SwapError::UnknownFilesystem(fs) => FsError::UnknownFilesystem(fs),
        }
    }
}

const PAGESIZE_MIN: u32 = 0xff6;
const PAGESIZE_MAX: u32 = 0xfff6;
const TOI_MAGIC_STRING: [u8; 8] = *b"\xed\xc3\x02\xe9\x98\x56\xe5\x0c";

pub const SWAP_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("swap"),
    usage: Some(UsageType::Other("swap")),
    probe_fn: |probe, magic| {
        probe_swap(probe, magic)
        .map_err(FsError::from)
        .map_err(BlockidError::from)
    },
    minsz: Some(40960), // 10 * 4096
    magics: &[
        BlockidMagic {
            magic: b"SWAP-SPACE",
            len: 10,
            b_offset: 0xff6,
        },
        BlockidMagic {
            magic: b"SWAPSPACE2",
            len: 10,
            b_offset: 0xff6,
        },
        BlockidMagic {
            magic: b"SWAP-SPACE",
            len: 10,
            b_offset: 0x1ff6,
        },
        BlockidMagic {
            magic: b"SWAPSPACE2",
            len: 10,
            b_offset: 0x1ff6,
        },
        BlockidMagic {
            magic: b"SWAP-SPACE",
            len: 10,
            b_offset: 0x3ff6,
        },
        BlockidMagic {
            magic: b"SWAPSPACE2",
            len: 10,
            b_offset: 0x3ff6,
        },
        BlockidMagic {
            magic: b"SWAP-SPACE",
            len: 10,
            b_offset: 0x7ff6,
        },
        BlockidMagic {
            magic: b"SWAPSPACE2",
            len: 10,
            b_offset: 0x7ff6,
        },
        BlockidMagic {
            magic: b"SWAP-SPACE",
            len: 10,
            b_offset: 0xfff6,
        },
        BlockidMagic {
            magic: b"SWAPSPACE2",
            len: 10,
            b_offset: 0xfff6,
        },
    ]
};

pub const SWSUSPEND_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("swapsuspend"),
    usage: Some(UsageType::Other("swapsuspend")),
    probe_fn: |probe, magic| {
        probe_swap(probe, magic)
        .map_err(FsError::from)
        .map_err(BlockidError::from)
    },
    minsz: Some(40960), // 10 * 4096
    magics: &[
        //BlockidMagic {
        //    magic: &TOI_MAGIC_STRING,
        //    len: 8,
        //    b_offset: 0,
        //},
        BlockidMagic {
            magic: b"S1SUSPEND",
            len: 9,
            b_offset: 0xff6,
        },
        BlockidMagic {
            magic: b"S2SUSPEND",
            len: 9,
            b_offset: 0xff6,
        },
        BlockidMagic {
            magic: b"ULSUSPEND",
            len: 9,
            b_offset: 0xff6,
        },
        BlockidMagic {
            magic: b"LINHIB0001",
            len: 9,
            b_offset: 0xff6,
        },
        BlockidMagic {
            magic: b"S1SUSPEND",
            len: 9,
            b_offset: 0x1ff6,
        },
        BlockidMagic {
            magic: b"S2SUSPEND",
            len: 9,
            b_offset: 0x1ff6,
        },
        BlockidMagic {
            magic: b"ULSUSPEND",
            len: 9,
            b_offset: 0x1ff6,
        },
        BlockidMagic {
            magic: b"LINHIB0001",
            len: 9,
            b_offset: 0x1ff6,
        },
        BlockidMagic {
            magic: b"S1SUSPEND",
            len: 9,
            b_offset: 0x3ff6,
        },
        BlockidMagic {
            magic: b"S2SUSPEND",
            len: 9,
            b_offset: 0x3ff6,
        },
        BlockidMagic {
            magic: b"ULSUSPEND",
            len: 9,
            b_offset: 0x3ff6,
        },
        BlockidMagic {
            magic: b"LINHIB0001",
            len: 9,
            b_offset: 0x3ff6,
        },
        BlockidMagic {
            magic: b"S1SUSPEND",
            len: 9,
            b_offset: 0x7ff6,
        },
        BlockidMagic {
            magic: b"S2SUSPEND",
            len: 9,
            b_offset: 0x7ff6,
        },
        BlockidMagic {
            magic: b"ULSUSPEND",
            len: 9,
            b_offset: 0x7ff6,
        },
        BlockidMagic {
            magic: b"LINHIB0001",
            len: 9,
            b_offset: 0x7ff6,
        },
        BlockidMagic {
            magic: b"S1SUSPEND",
            len: 9,
            b_offset: 0xfff6,
        },
        BlockidMagic {
            magic: b"S2SUSPEND",
            len: 9,
            b_offset: 0xfff6,
        },
        BlockidMagic {
            magic: b"ULSUSPEND",
            len: 9,
            b_offset: 0xfff6,
        },
        BlockidMagic {
            magic: b"LINHIB0001",
            len: 9,
            b_offset: 0xfff6,
        },
    ]
};

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct SwapHeaderV1 {
    pub version: U32<LittleEndian>,
    pub lastpage: U32<LittleEndian>,
    pub nr_badpages: U32<LittleEndian>,
    pub uuid: [u8; 16],
    pub volume: [u8; 16],
    pub padding: [u8; 117],
    pub badpages: U32<LittleEndian>,
}

pub fn probe_swap(
        probe: &mut BlockidProbe, 
        magic: BlockidMagic
    ) -> Result<(), SwapError> 
{
    let header: SwapHeaderV1 = read_as(&mut probe.file, 1024)?;

    

    println!("{:X?}", header);

    return Ok(());
}
