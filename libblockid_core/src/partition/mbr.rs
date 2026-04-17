use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned,
    byteorder::{LittleEndian, U32},
    transmute,
};

use crate::{
    error::Error,
    io::{BlockIo, Reader},
    partition::aix::AIX_MAGIC,
    probe::{Magic, PartTableInfo},
    std::fmt,
};

#[derive(Debug)]
pub enum MbrError {
    ProbablyAix,
    ProbablyGPT,
    ProbablyVFAT,
    ProbablyEXFAT,
    ProbablyNTFS,
    MissingBootIndicator,
    BadPrimaryExtendedOffset,
    InvalidExtendedSignature,
}

impl fmt::Display for MbrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MbrError::ProbablyAix => write!(f, "Partition table has AIX magic signature"),
            MbrError::ProbablyGPT => write!(f, "Partition table looks like GPT"),
            MbrError::ProbablyVFAT => write!(f, "Partition table looks like VFAT"),
            MbrError::ProbablyEXFAT => write!(f, "Partition table looks like EXFAT"),
            MbrError::ProbablyNTFS => write!(f, "Partition table looks like NTFS"),
            MbrError::MissingBootIndicator => {
                write!(f, "Missing boot indicator in partition entry")
            }
            MbrError::BadPrimaryExtendedOffset => {
                write!(f, "Bad offset in primary extended partition")
            }
            MbrError::InvalidExtendedSignature => {
                write!(f, "Extended partition is missing a valid signature")
            }
        }
    }
}

impl<IO: BlockIo> From<MbrError> for Error<IO> {
    fn from(e: MbrError) -> Self {
        Error::Mbr(e)
    }
}

pub const MBR_MAGICS: Option<&'static [Magic]> = Some(&[Magic {
    magic: b"\x55\xAA",
    len: 2,
    b_offset: 510,
}]);

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout)]
pub struct MbrTable {
    pub boot_code1: [u8; 218],
    pub disk_timestamp: [u8; 6],
    pub boot_code2: [u8; 216],
    pub disk_id: [u8; 4],
    pub state: [u8; 2],
    pub partition_entries: [MbrPartitionEntry; 4],
    pub boot_signature: [u8; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct MbrPartitionEntry {
    pub boot_ind: u8,   /* 0x80 - active */
    pub begin_head: u8, /* begin CHS */
    pub begin_sector: u8,
    pub begin_cylinder: u8,
    pub sys_ind: u8,  /* https://en.wikipedia.org/wiki/Partition_type */
    pub end_head: u8, /* end CHS */
    pub end_sector: u8,
    pub end_cylinder: u8,
    pub start_sect: U32<LittleEndian>,
    pub nr_sects: U32<LittleEndian>,
}

pub fn probe_mbr<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    mag: Magic,
) -> Result<PartTableInfo, Error<IO>> {
    let buf: [u8; size_of::<MbrTable>()] = reader.read_exact_at(offset).map_err(Error::io)?;

    let mbr_pt: MbrTable = transmute!(buf);

    if mbr_pt.boot_code1[0..3] == AIX_MAGIC {
        return Err(MbrError::ProbablyAix.into());
    }

    todo!()
}
