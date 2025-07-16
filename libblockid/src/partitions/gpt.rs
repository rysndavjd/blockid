use core::{any::Any, fmt};
use alloc::vec::Vec;

#[cfg(feature = "std")]
use std::io::{Error as IoError, Seek, Read, ErrorKind};

#[cfg(not(feature = "std"))]
use crate::nostd_io::{NoStdIoError as IoError, Read, Seek, ErrorKind};

use bitflags::bitflags;
use zerocopy::{byteorder::{LittleEndian, U16, U32, U64}, Ref, 
    FromBytes, Immutable, IntoBytes, Unaligned};
use uuid::Uuid;

use crate::{
    checksum::{get_crc32_iso_hdlc,
    verify_crc32_iso_hdlc}, filesystems::volume_id::VolumeId32, from_file, partitions::PtError, read_sector_at, read_vec_at, BlockidError, BlockidIdinfo, BlockidMagic, BlockidProbe, BlockidUUID, PartEntryAttributes, PartEntryType, PartTableResults, PartitionResults, ProbeResult, PtType, UsageType
};

#[derive(Debug)]
pub enum GptPtError {
    IoError(IoError),
    UnknownPartitionTable(&'static str),
    GptPTHeaderError(&'static str),
}

impl fmt::Display for GptPtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GptPtError::IoError(e) => write!(f, "I/O operation failed: {e}"),
            GptPtError::UnknownPartitionTable(e) => write!(f, "Not an GPT table superblock: {e}"),
            GptPtError::GptPTHeaderError(e) => write!(f, "GPT table header error: {e}"),
        }
    }
}

impl From<GptPtError> for PtError {
    fn from(err: GptPtError) -> Self {
        match err {
            GptPtError::IoError(e) => PtError::IoError(e),
            GptPtError::UnknownPartitionTable(pt) => PtError::UnknownPartition(pt),
            GptPtError::GptPTHeaderError(pt) => PtError::InvalidHeader(pt),
        }
    }
}

impl From<IoError> for GptPtError {
    fn from(err: IoError) -> Self {
        GptPtError::IoError(err)
    }
}

pub const GPT_PT_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("gpt"),
    usage: Some(UsageType::PartitionTable),
    minsz: None,
    probe_fn: |probe, magic| {
        probe_gpt_pt(probe, magic)
        .map_err(PtError::from)
        .map_err(BlockidError::from)
    },
    magics: None
};

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct EfiGuid {
    time_low: U32<LittleEndian>,
    time_mid: U16<LittleEndian>,
    time_hi_and_version: U16<LittleEndian>,
    clock_seq_hi: u8,
    clock_seq_low: u8,
    node: [u8; 6],
}

impl From<EfiGuid> for Uuid {
    fn from(uuid: EfiGuid) -> Self {
        Uuid::from_fields(
            u32::from(uuid.time_low), 
            u16::from(uuid.time_mid), 
            u16::from(uuid.time_hi_and_version), 
            &[uuid.clock_seq_hi, uuid.clock_seq_low, 
                uuid.node[0], uuid.node[1], uuid.node[2], 
                uuid.node[3], uuid.node[4], uuid.node[5]]
        )
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct GptTable {
    pub signature: U64<LittleEndian>,
    pub revision: U32<LittleEndian>,
    pub header_size: U32<LittleEndian>,
    pub header_crc32: U32<LittleEndian>,

    pub reserved1: U32<LittleEndian>,
    
    pub my_lba: U64<LittleEndian>,
    pub alternate_lba: U64<LittleEndian>,
    pub first_usable_lba: U64<LittleEndian>,
    pub last_usable_lba: U64<LittleEndian>,

    pub disk_guid: EfiGuid,

    pub partition_entries_lba: U64<LittleEndian>,
    pub num_partition_entries: U32<LittleEndian>,
    pub sizeof_partition_entry: U32<LittleEndian>,
    pub partition_entry_array_crc32: U32<LittleEndian>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct GptEntry {
    partition_type_guid: EfiGuid,
    unique_partition_guid: EfiGuid,
    starting_lba: U64<LittleEndian>,
    ending_lba: U64<LittleEndian>,

    attributes: U64<LittleEndian>,
    partition_name: [U16<LittleEndian>; 36]
}

const GPT_HEADER_SIGNATURE: u64 = 0x5452415020494645;

fn get_lba_buffer(probe: &mut BlockidProbe, lba: u64, size: usize) -> Result<Vec<u8>, IoError> {
    return Ok(read_vec_at(&mut probe.file, lba * probe.sector_size, size)?)
}

fn last_lba(probe: &mut BlockidProbe) -> Option<u64> {
    let sz = probe.size;
    let ssz = probe.sector_size;

    if sz < ssz {
        return None;
    }

    return Some((sz / ssz) - 1);
}

fn is_pmbr_valid(probe: &mut BlockidProbe) -> bool {
    false
}

fn get_gpt_header(probe: &mut BlockidProbe, lba: u64, last_lba: u64) -> Result<(),GptPtError>{
    let ssz = probe.sector_size;

    let raw = get_lba_buffer(probe, lba, ssz as usize)?;
    
    println!("raw {:?}", raw.len());

    let header: GptTable = GptTable::read_from_bytes(&raw[..92])
        .map_err(|_| IoError::new(ErrorKind::InvalidData, "Unable to map bytes to GPT partition table"))?;

    if u64::from(header.signature) != GPT_HEADER_SIGNATURE {
        return Err(GptPtError::GptPTHeaderError("Invalid GPT header signature"));
    }

    let hsz = u64::from(header.header_size);

    if hsz > ssz || hsz < size_of::<GptTable>() as u64 {
        return Err(GptPtError::GptPTHeaderError("GPT header size too large"));
    }
    
    //let mut janky = header.clone();
    //let mut header_bytes = *janky.as_bytes();
    //header_bytes[16..20].fill(0);

    //let crc = get_crc32_iso_hdlc(header_bytes);

    //if verify_crc32_iso_hdlc(header_bytes, crc) {
    //    return Err(GptPtError::GptPTHeaderError("Corrupted GPT header"));
    //}

    if u64::from(header.my_lba) != lba {
        return Err(GptPtError::GptPTHeaderError("GPT->MyLBA mismatch with real position"));
    }

    let fu = u64::from(header.first_usable_lba);
    let lu = u64::from(header.last_usable_lba);

    if lu < fu || fu > last_lba || lu > last_lba {
        return Err(GptPtError::GptPTHeaderError("GPT->{First,Last}UsableLBA out of range"));
    }

    if fu < lba && lba < lu {
        return Err(GptPtError::GptPTHeaderError("GPT header is inside usable area"));
    }

    let esz = u64::from(header.num_partition_entries) *
        u64::from(header.sizeof_partition_entry);

    if esz == 0 || esz >= u64::from(u32::MAX) ||
        u32::from(header.sizeof_partition_entry) != size_of::<GptEntry>() as u32 {
        return Err(GptPtError::GptPTHeaderError("GPT entries undefined"));
    }

    let entry_buffers: &[u8] = &get_lba_buffer(probe, u64::from(header.partition_entries_lba), esz as usize)?;
    let count = entry_buffers.len() / size_of::<GptEntry>();
    
    println!("count: {:?}", count);
    println!("num_partition_entries: {:?}", u32::from(header.num_partition_entries));

    if count as u32 != u32::from(header.num_partition_entries) {
        panic!("AHHHHHH")
    }

    let entries = Ref::<_, [GptEntry]>::from_bytes_with_elems(entry_buffers, count)
        .map(|r| zerocopy::Ref::into_ref(r))
        .map_err(|_| IoError::new(ErrorKind::InvalidData, "Unable to map bytes to array of GPT partition entries"))?;
    
    println!("Entries: {:?}", entries);

    return Ok(());
}

pub fn probe_gpt_pt(
        probe: &mut BlockidProbe, 
        _mag: BlockidMagic
    ) -> Result<(), GptPtError> 
{   
    let lastlba = match last_lba(probe) {
        Some(t) => t,
        None => return Err(GptPtError::GptPTHeaderError("Unable to get last lba"))
    };


    //let ssf = probe.sector_size / 512;

    //let fu = 

    get_gpt_header(probe, 1, lastlba)?;

    return Ok(());
}