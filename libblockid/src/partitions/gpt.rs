use std::io::{Error as IoError, ErrorKind, Seek, Read};

use zerocopy::{byteorder::{LittleEndian, U16, U32, U64}, 
    FromBytes, Immutable, IntoBytes, Unaligned, KnownLayout};
use uuid::Uuid;

use crate::{
    checksum::verify_crc32_iso_hdlc, partitions::{dos::{DosTable, MbrPartitionType}, PtError}, read_sector_at, read_vec_at, util::decode_utf16_lossy_from, BlockidError, BlockidIdinfo, BlockidMagic, BlockidProbe, BlockidUUID, Endianness, PartEntryAttributes, PartEntryType, PartTableResults, PartitionResults, ProbeFlags, ProbeResult, PtType, UsageType
};

#[derive(Debug)]
pub enum GptPtError {
    IoError(IoError),
    UnknownPartitionTable(&'static str),
    GptPTHeaderError(&'static str),
}

impl std::fmt::Display for GptPtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    name: Some("gpt_pt"),
    usage: Some(UsageType::PartitionTable),
    minsz: None,
    probe_fn: |probe, magic| {
        probe_gpt_pt(probe, magic)
        .map_err(PtError::from)
        .map_err(BlockidError::from)
    },
    magics: None
};

pub const PMBR_PT_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("pmbr"),
    usage: Some(UsageType::PartitionTable),
    minsz: None,
    probe_fn: |probe, magic| {
        probe_pmbr_pt(probe, magic)
        .map_err(PtError::from)
        .map_err(BlockidError::from)
    },
    magics: Some(&[
        BlockidMagic {
            magic: b"\x55\xAA",
            len: 2,
            b_offset: 510,
        }
    ])
};

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, PartialEq)]
pub struct EfiGuid {
    time_low: U32<LittleEndian>,
    time_mid: U16<LittleEndian>,
    time_hi_and_version: U16<LittleEndian>,
    clock_seq_hi: u8,
    clock_seq_low: u8,
    node: [u8; 6],
}

impl EfiGuid {
    const ZERO: EfiGuid = EfiGuid {
        time_low: U32::new(0),
        time_mid: U16::new(0),
        time_hi_and_version: U16::new(0),
        clock_seq_hi: 0,
        clock_seq_low: 0,
        node: [0u8; 6]
    };

    fn is_zero(&self) -> bool {
        *self == EfiGuid::ZERO
    }
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
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout)]
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
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout)]
pub struct GptEntry {
    partition_type_guid: EfiGuid,
    unique_partition_guid: EfiGuid,
    starting_lba: U64<LittleEndian>,
    ending_lba: U64<LittleEndian>,

    attributes: U64<LittleEndian>,
    partition_name: [u8; 72]
}

impl GptTable {
    const HEADER_SIGNATURE: u64 = 0x5452415020494645;
    const HEADER_SIGNATURE_STR: &[u8] = b"EFI PART";
}

fn get_lba_buffer<R: Seek+Read>(file: &mut R, ssz: u64, lba: u64, offset: u64) -> Result<Vec<u8>, IoError> {
    return Ok(read_vec_at(file, (lba * ssz) + offset, ssz as usize)?)
}

fn last_lba(probe: &mut BlockidProbe) -> Option<u64> {
    let sz = probe.size;
    let ssz = probe.sector_size;

    if sz < ssz {
        return None;
    }

    return Some((sz / ssz) - 1);
}

fn is_pmbr_valid(probe: &mut BlockidProbe) -> Result<bool, GptPtError> {
    if probe.flags.contains(ProbeFlags::FORCE_GPT_PMBR) {
        return Ok(true);
    }

    let data = read_sector_at(&mut probe.file, probe.offset / 512)?;
    
    let mbr = DosTable::ref_from_bytes(&data)
        .map_err(|_| IoError::new(ErrorKind::InvalidData, "Unable to map bytes to MBR partition table"))?;

    for partition in mbr.partition_entries {
        if partition.sys_ind == MbrPartitionType::MBR_GPT_PARTITION {
            return Ok(true);
        }
    }

    return Ok(false);
}

fn get_gpt_header<R: Seek+Read>(file: &mut R, ssz: u64, lba: u64, last_lba: u64, offset: u64) -> Result<ProbeResult, GptPtError>{
    let raw = get_lba_buffer(file, ssz, lba, offset)?;
    
    let header = GptTable::ref_from_bytes(&raw[..92])
        .map_err(|_| IoError::new(ErrorKind::InvalidData, "Unable to map bytes to GPT partition table"))?;

    if u64::from(header.signature) != GptTable::HEADER_SIGNATURE {
        return Err(GptPtError::GptPTHeaderError("Invalid GPT header signature"));
    }

    let hsz = u64::from(header.header_size);

    if hsz > ssz || hsz < size_of::<GptTable>() as u64 {
        return Err(GptPtError::GptPTHeaderError("GPT header size too large"));
    }
    
    let stored_crc = u32::from(header.header_crc32);

    let mut header_bytes = raw[..size_of::<GptTable>()].to_vec();
    header_bytes[16..20].fill(0);

    if !verify_crc32_iso_hdlc(&header_bytes, stored_crc) {
        return Err(GptPtError::GptPTHeaderError("Corrupted GPT header"));
    }

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

    let entry_buffers: &[u8] = &get_lba_buffer(file, u64::from(header.partition_entries_lba), esz, offset)?;
    let count = entry_buffers.len() / size_of::<GptEntry>();
    
    if count as u32 != u32::from(header.num_partition_entries) {
        return Err(GptPtError::GptPTHeaderError("Calculated partition count not equal to header count"));
    }
    
    let ssf = ssz / 512;

    let partitions: Vec<PartitionResults> = (1..=count)
        .filter_map(|partno| {
            let start_off = (partno - 1) * 128;
            let end_off = partno * 128;

            let entry = GptEntry::ref_from_bytes(&entry_buffers[start_off..end_off]).ok()?;

            if entry.unique_partition_guid.is_zero() {
                return None;
            } else {
                return Some((partno, entry));
            }
        })
        .filter_map(|(entry_no, entry)| {
            let start = u64::from(entry.starting_lba);
            let size = u64::from(entry.ending_lba) -
                u64::from(entry.starting_lba) + 1;

            if start < fu || start + size - 1 > lu {
                return None;
            }

            let name = if entry.partition_name != [0u8; 72] {
                Some(decode_utf16_lossy_from(&entry.partition_name, Endianness::Little).to_string())
            } else {
                None
            };
            
            return Some(
                PartitionResults { 
                    offset: Some(start * ssf), 
                    size: Some(size * ssf), 
                    partno: Some(entry_no as u64), 
                    part_uuid: Some(BlockidUUID::Uuid(Uuid::from(entry.unique_partition_guid))), 
                    name,
                    entry_type: Some(PartEntryType::Uuid(Uuid::from(entry.partition_type_guid))), 
                    entry_attributes: Some(PartEntryAttributes::Gpt(u64::from(entry.attributes))) 
                }
            );
        })
    .collect();

    return Ok(
        ProbeResult::PartTable(
            PartTableResults { 
                offset: Some(offset), 
                pt_type: Some(PtType::Gpt), 
                pt_uuid: Some(BlockidUUID::Uuid(Uuid::from(header.disk_guid))), 
                sbmagic: Some(GptTable::HEADER_SIGNATURE_STR), 
                sbmagic_offset: Some(ssz * lba), 
                partitions: Some(partitions) 
            }
        )
    );
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

    let result = match get_gpt_header(&mut probe.file, probe.sector_size, 1, lastlba, probe.offset) {
        Ok(t) => t,
        Err(_) => {
            get_gpt_header(&mut probe.file, probe.sector_size, lastlba, lastlba, probe.offset)?
        }
    };

    probe.push_result(result);

    return Ok(());
}

pub fn probe_pmbr_pt(
        probe: &mut BlockidProbe, 
        _mag: BlockidMagic
    ) -> Result<(), GptPtError> 
{
    let lastlba = match last_lba(probe) {
        Some(t) => t,
        None => return Err(GptPtError::GptPTHeaderError("Unable to get last lba"))
    };

    if !is_pmbr_valid(probe)? {
        return Err(GptPtError::UnknownPartitionTable("PT does not contain PMBR"));
    }
    
    if get_gpt_header(&mut probe.file, probe.sector_size, 1, lastlba, probe.offset).is_err() &&
        get_gpt_header(&mut probe.file, probe.sector_size, lastlba, lastlba, probe.offset).is_err() {
        return Ok(());
    }

    return Err(GptPtError::UnknownPartitionTable("Not a GPT+PMBR"));
}