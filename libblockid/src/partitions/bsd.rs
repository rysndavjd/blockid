use core::fmt::{self, Debug};
use alloc::{vec::Vec};

#[cfg(feature = "std")]
use std::io::{Error as IoError, Seek, Read, ErrorKind};

#[cfg(not(feature = "std"))]
use crate::nostd_io::{NoStdIoError as IoError, Read, Seek, ErrorKind};

use bitflags::bitflags;
use zerocopy::{byteorder::LittleEndian, byteorder::U32, byteorder::U16, 
    transmute, FromBytes, Immutable, IntoBytes, Unaligned};

use crate::{
    BlockidError, BlockidIdinfo, BlockidMagic, BlockidProbe, BlockidUUID,
    PartEntryAttributes, PartEntryType, PartTableResults, PartitionResults,
    ProbeResult, PtType, UsageType, from_file, read_sector_at, filesystems::{
    volume_id::VolumeId32}, partitions::PtError,
};

fn mag_sector(mag: &BlockidMagic) -> u64 {
    (0 / 2) + (mag.b_offset >> 9)
}

fn mag_offset(mag: &BlockidMagic) -> u64 {
    (0 << 10) + mag.b_offset
}

fn mag_lastoffset(mag: &BlockidMagic) -> u64 {
    mag_offset(mag) - (mag_sector(mag) << 9)
}

#[derive(Debug)]
pub enum BsdError {
    IoError(IoError),
    BsdHeaderError(&'static str),
    UnknownFilesystem(&'static str),
}

impl fmt::Display for BsdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BsdError::IoError(e) => write!(f, "I/O operation failed: {}", e),
            BsdError::BsdHeaderError(e) => write!(f, "BSD disklabel header error: {}", e),
            BsdError::UnknownFilesystem(e) => write!(f, "Not an BSD disklabel: {}", e),
        }
    }
}

impl From<BsdError> for PtError {
    fn from(err: BsdError) -> Self {
        match err {
            BsdError::IoError(e) => PtError::IoError(e),
            BsdError::BsdHeaderError(e) => PtError::InvalidHeader(e),
            BsdError::UnknownFilesystem(e) => PtError::UnknownPartition(e),
        }
    }
}

impl From<IoError> for BsdError {
    fn from(err: IoError) -> Self {
        BsdError::IoError(err)
    }
}

pub const BSD_PT_IDINFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("bsd"),
    usage: Some(UsageType::PartitionTable),
    probe_fn: |probe, magic| {
        probe_bsd_pt(probe, magic)
        .map_err(PtError::from)
        .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[
        BlockidMagic {
            magic: b"\x57\x45\x56\x82",
            len: 4,
            b_offset: 512,
        },
        BlockidMagic {
            magic: b"\x57\x45\x56\x82",
            len: 4,
            b_offset: 64,
        },
        BlockidMagic {
            magic: b"\x57\x45\x56\x82",
            len: 4,
            b_offset: 128,
        },
    ])
};

const BSD_MAXPARTITIONS: usize = 16;

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct BsdPartition {
    p_size: U32<LittleEndian>,
    p_offset: U32<LittleEndian>,
    p_fsize: U32<LittleEndian>,
    p_fstype: u8,
    p_frag: u8,
    p_cpg: U16<LittleEndian>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct BsdDType(U16<LittleEndian>);

impl BsdDType {
    pub const BSD_DTYPE_SMD: Self = Self(U16::new(1));
    pub const BSD_DTYPE_MSCP: Self = Self(U16::new(2));
    pub const BSD_DTYPE_DEC: Self = Self(U16::new(3));
    pub const BSD_DTYPE_SCSI: Self = Self(U16::new(4));
    pub const BSD_DTYPE_ESDI: Self = Self(U16::new(5));
    pub const BSD_DTYPE_ST506: Self = Self(U16::new(6));
    pub const BSD_DTYPE_HPIB: Self = Self(U16::new(7));
    pub const BSD_DTYPE_HPFL: Self = Self(U16::new(8));
    pub const BSD_DTYPE_FLOPPY: Self = Self(U16::new(10));
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct BsdDSubType(U16<LittleEndian>);

impl BsdDSubType {
    pub const BSD_DSTYPE_INDOSPART: Self = Self(U16::new(0x8));
    pub const BSD_DSTYPE_GEOMETRY: Self = Self(U16::new(0x10));
    
    pub fn bsd_dstype_dospart(
            partno: u8
        ) -> u8
    {
        partno & 3
    }

    pub fn from_u16(
            bytes: u16
        ) -> Self 
    {
        Self(U16::new(bytes))
    }
    
    pub fn as_u16(
            &self
        ) -> u16
    {
        u16::from(self.0)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct BsdDiskLabel {
    d_magic: U32<LittleEndian>,
    d_type: BsdDType,
    d_subtype: BsdDSubType,
    d_typename: [U32<LittleEndian>; 16],
    d_packname: [U32<LittleEndian>; 16],

    d_secsize: U32<LittleEndian>,
    d_nsectors: U32<LittleEndian>,
    d_ntracks: U32<LittleEndian>,
    d_ncylinders: U32<LittleEndian>,
    d_secpercyl: U32<LittleEndian>,
    d_secperunit: U32<LittleEndian>,
    
    d_sparespertrack: U16<LittleEndian>,
    d_sparespercyl: U16<LittleEndian>,

    d_acylinders: U32<LittleEndian>,

    d_rpm: U16<LittleEndian>,
    d_interleave: U16<LittleEndian>,
    d_trackskew: U16<LittleEndian>,
    d_cylskew: U16<LittleEndian>,
    d_headswitch: U32<LittleEndian>,
    d_trkseek: U32<LittleEndian>,
    d_flags: U32<LittleEndian>,
    d_drivedata: [U32<LittleEndian>; 5],
    d_spare: [U32<LittleEndian>; 5],
    d_magic2: U32<LittleEndian>,
    d_checksum: U16<LittleEndian>,

    d_npartitions: U16<LittleEndian>,
    d_bbsize: U32<LittleEndian>,
    d_sbsize: U32<LittleEndian>,
    d_partitions: [BsdPartition; BSD_MAXPARTITIONS],
}

impl BsdDiskLabel {

}

fn bsd_checksum(
        label: BsdDiskLabel
    ) -> u16
{
    let raw: Vec<u16> = label.as_bytes()
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();

    let result = raw.iter().fold(0u16, |acc, &x| acc ^ x);

    return result ^ u16::from(label.d_checksum);
}

/*
 * BSD disk label is pain in the ass to develop on linux and
 * will finish this when I figure out a workflow of creating
 * correct disk labels as Gnu Parted seems to make invaild bsd 
 * disk labels
 */

 pub fn probe_bsd_pt(
        probe: &mut BlockidProbe,
        mag: BlockidMagic,
    ) -> Result<(), BsdError> 
{
    //let data = read_sector_at(&mut probe.file, mag_sector(&mag))?;

    todo!();
    //return Ok(());
}
