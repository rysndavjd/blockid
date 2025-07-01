use std::{fs::File, io};

use zerocopy::{FromBytes, IntoBytes, Unaligned, Immutable};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    filesystems::FsError, read_as, read_buffer, BlockidError, BlockidIdinfo, 
    BlockidMagic, BlockidProbe, BlockidUUID, BlockidVersion, Endianness, 
    FilesystemResults, FsType, ProbeResult, UsageType
};

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

//const PAGESIZE_MIN: u32 = 0xff6;
//const PAGESIZE_MAX: u32 = 0xfff6;
const TOI_MAGIC_STRING: [u8; 8] = *b"\xed\xc3\x02\xe9\x98\x56\xe5\x0c";

pub const SWAP_V0_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("swap"),
    usage: Some(UsageType::Other("swap")),
    probe_fn: |probe, magic| {
        probe_swap_v0(probe, magic)
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
            magic: b"SWAP-SPACE",
            len: 10,
            b_offset: 0x1ff6,
        },
        BlockidMagic {
            magic: b"SWAP-SPACE",
            len: 10,
            b_offset: 0x3ff6,
        },
        BlockidMagic {
            magic: b"SWAP-SPACE",
            len: 10,
            b_offset: 0x7ff6,
        },
        BlockidMagic {
            magic: b"SWAP-SPACE",
            len: 10,
            b_offset: 0xfff6,
        },
    ]
};

pub const SWAP_V1_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("swap"),
    usage: Some(UsageType::Other("swap")),
    probe_fn: |probe, magic| {
        probe_swap_v1(probe, magic)
        .map_err(FsError::from)
        .map_err(BlockidError::from)
    },
    minsz: Some(40960), // 10 * 4096
    magics: &[
        BlockidMagic {
            magic: b"SWAPSPACE2",
            len: 10,
            b_offset: 0xff6,
        },
        BlockidMagic {
            magic: b"SWAPSPACE2",
            len: 10,
            b_offset: 0x1ff6,
        },
        BlockidMagic {
            magic: b"SWAPSPACE2",
            len: 10,
            b_offset: 0x3ff6,
        },
        BlockidMagic {
            magic: b"SWAPSPACE2",
            len: 10,
            b_offset: 0x7ff6,
        },
    ]
};

pub const SWSUSPEND_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("swapsuspend"),
    usage: Some(UsageType::Other("swapsuspend")),
    probe_fn: |probe, magic| {
        probe_swsuspend(probe, magic)
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
    pub version: [u8; 4],
    pub lastpage: [u8; 4],
    pub nr_badpages: [u8; 4],
    pub uuid: [u8; 16],
    pub volume: [u8; 16],
    pub padding: [u8; 117],
    pub badpages: [u8; 4],
}

fn swap_get_info(
        magic: BlockidMagic,
        name: &'static str,
        header: SwapHeaderV1
    ) -> Result<(Endianness, u64, u64, u64, &'static str), SwapError> 
{
    let endianness = if u32::from_be_bytes(header.version) == 1 {
        Endianness::Big
    } else {
        Endianness::Little
    };

    let pagesize = magic.b_offset + magic.len as u64;

    let lastpage = if endianness == Endianness::Little {
        u32::from_le_bytes(header.lastpage) as u64
    } else {
        u32::from_be_bytes(header.lastpage) as u64
    };

    let fs_size = pagesize * lastpage;

    let fs_last_block = lastpage + 1;

    return Ok((endianness, pagesize, fs_size, fs_last_block, name));
}

pub fn probe_swap_v0(
        probe: &mut BlockidProbe, 
        magic: BlockidMagic
    ) -> Result<(), SwapError> 
{
    let check = read_buffer::<8, File>(&mut probe.file, 1024)?;

    if check == TOI_MAGIC_STRING {
        return Err(SwapError::UnknownFilesystem("TuxOnIce signature detected"));
    }

    if magic.magic == b"SWAP-SPACE" {
        let header: SwapHeaderV1 = read_as(&mut probe.file, 1024)?;
        
        let (endian, pagesize, fs_size, fs_last_block, 
            name) = swap_get_info(magic, "Swap V0", header)?;
    
        probe.push_result(ProbeResult::Filesystem(
                FilesystemResults { 
                    fs_type: Some(FsType::LinuxSwap), 
                    sec_type: None, 
                    label: None, 
                    fs_uuid: None, 
                    log_uuid: None, 
                    ext_journal: None, 
                    fs_creator: None, 
                    usage: Some(UsageType::Other(name)), 
                    version: Some(BlockidVersion::Number(0)), 
                    sbmagic: Some(magic.magic), 
                    sbmagic_offset: Some(magic.b_offset), 
                    fs_size: Some(fs_size), 
                    fs_last_block: Some(fs_last_block), 
                    fs_block_size: Some(pagesize), 
                    block_size: None,
                    endianness: Some(endian),
                }
            )
        );
        return Ok(());
    } else {
        return Err(SwapError::UnknownFilesystem("Linux Swap v1 detected"));
    }
}

pub fn probe_swap_v1(
        probe: &mut BlockidProbe, 
        magic: BlockidMagic
    ) -> Result<(), SwapError> 
{
    let check = read_buffer::<8, File>(&mut probe.file, 1024)?;

    if check == TOI_MAGIC_STRING {
        return Err(SwapError::UnknownFilesystem("TuxOnIce signature detected"));
    }

    if magic.magic == b"SWAPSPACE2" {
        let header: SwapHeaderV1 = read_as(&mut probe.file, 1024)?;
        
        let (endian, pagesize, fs_size, fs_last_block, 
            name) = swap_get_info(magic, "Swap V1", header)?;
        
        let uuid = Uuid::from_bytes(header.uuid);

        let label: Option<String> = if header.volume[0] != 0 {
            Some(String::from_utf8_lossy(&header.volume).trim_end_matches('\0').to_string())
        } else {
            None
        };

        probe.push_result(ProbeResult::Filesystem(
                FilesystemResults { 
                    fs_type: Some(FsType::LinuxSwap), 
                    sec_type: None, 
                    label: label, 
                    fs_uuid: Some(BlockidUUID::Standard(uuid)), 
                    log_uuid: None, 
                    ext_journal: None, 
                    fs_creator: None, 
                    usage: Some(UsageType::Other(name)), 
                    version: Some(BlockidVersion::Number(1)), 
                    sbmagic: Some(magic.magic), 
                    sbmagic_offset: Some(magic.b_offset), 
                    fs_size: Some(fs_size), 
                    fs_last_block: Some(fs_last_block), 
                    fs_block_size: Some(pagesize), 
                    block_size: None,
                    endianness: Some(endian),
                }
            )
        );
        return Ok(());
    } else {
        return Err(SwapError::UnknownFilesystem("Linux Swap v0 detected"));
    }
}

pub fn probe_swsuspend(
        probe: &mut BlockidProbe, 
        magic: BlockidMagic
    ) -> Result<(), SwapError> 
{
    let header: SwapHeaderV1 = read_as(&mut probe.file, 1024)?;

    let (endian, pagesize, fs_size, fs_last_block,
         name) = if magic.magic == b"S1SUSPEND" {
        swap_get_info(magic, "s1suspend", header)?
    } else if magic.magic == b"S2SUSPEND" {
        swap_get_info(magic, "s2suspend", header)?
    } else if magic.magic == b"ULSUSPEND" {
        swap_get_info(magic, "ulsuspend", header)?
    } else if magic.magic == TOI_MAGIC_STRING {
        swap_get_info(magic, "Tux On Ice",  header)?
    } else if magic.magic == b"LINHIB0001" {
        swap_get_info(magic, "linhib0001", header)?
    } else {
        return Err(SwapError::UnknownFilesystem("Suspend magic not found"));
    };

    probe.push_result(ProbeResult::Filesystem(
            FilesystemResults { 
                fs_type: Some(FsType::LinuxSwap), 
                sec_type: None, 
                label: None, 
                fs_uuid: None, 
                log_uuid: None, 
                ext_journal: None, 
                fs_creator: None, 
                usage: Some(UsageType::Other(name)), 
                version: None, 
                sbmagic: Some(magic.magic), 
                sbmagic_offset: Some(magic.b_offset), 
                fs_size: Some(fs_size), 
                fs_last_block: Some(fs_last_block), 
                fs_block_size: Some(pagesize), 
                block_size: None,
                endianness: Some(endian),
            }
        )
    );

    return Ok(());
}