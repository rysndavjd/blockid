use uuid::Uuid;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, LittleEndian, U16, U32, U64, Unaligned,
    transmute_ref,
};

use crate::{BlockInfo, BlockIo, error::Error, io::Reader, probe::Magic};

#[derive(Debug)]
pub enum GptError {
    UnableToGetSectorSize,
}

impl core::fmt::Display for GptError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GptError::UnableToGetSectorSize => {
                write!(f, "Unable to get sector size of GPT partition table")
            }
        }
    }
}

impl<E: core::fmt::Debug> From<GptError> for Error<E> {
    fn from(e: GptError) -> Self {
        Error::Gpt(e)
    }
}

pub const GPT_MAGICS: Option<&'static [Magic]> = None;
/// The offset used that is read off the disk to find the GPT header and its block size.
pub const GPT_DETECT_OFFSET: usize = 32768;

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
        node: [0u8; 6],
    };

    fn is_zero(&self) -> bool {
        self == &EfiGuid::ZERO
    }
}

impl From<EfiGuid> for Uuid {
    fn from(uuid: EfiGuid) -> Self {
        Uuid::from_fields(
            u32::from(uuid.time_low),
            u16::from(uuid.time_mid),
            u16::from(uuid.time_hi_and_version),
            &[
                uuid.clock_seq_hi,
                uuid.clock_seq_low,
                uuid.node[0],
                uuid.node[1],
                uuid.node[2],
                uuid.node[3],
                uuid.node[4],
                uuid.node[5],
            ],
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
    partition_name: [u8; 72],
}

impl GptTable {
    const SIGNATURE: u64 = 0x5452415020494645;
    const SIGNATURE_STR: &[u8] = b"EFI PART";
}

fn probe_gpt<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    _: Magic,
) -> Result<BlockInfo, Error<IO::Error>> {
    let buf: [u8; GPT_DETECT_OFFSET] = reader.read_exact_at(offset).map_err(Error::Io)?;

    #[cfg(not(feature = "os_calls"))]
    let ssz = buf
        .chunks_exact(GptTable::SIGNATURE_STR.len())
        .enumerate()
        .take_while(|(i, _)| i * GptTable::SIGNATURE_STR.len() < GPT_DETECT_OFFSET)
        .find_map(|(i, raw)| {
            if raw == GptTable::SIGNATURE_STR {
                Some(i * GptTable::SIGNATURE_STR.len())
            } else {
                None
            }
        })
        .ok_or(GptError::UnableToGetSectorSize)?;

    // let ssz = crate::topology::physical_sector_size(file)

    let sb: &GptTable = transmute_ref!(&buf);

    todo!()
}
