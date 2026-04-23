use uuid::Uuid;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, LittleEndian, U16, U32, U64, Unaligned,
};

use crate::{probe::Magic, std::fmt};

#[derive(Debug)]
pub enum GptError {}

impl fmt::Display for GptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // GptError::IoError(e) => write!(f, "I/O operation failed: {e}"),
            _ => todo!(),
        }
    }
}

pub const GPT_MAGICS: Option<&'static [Magic]> = None;

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
    const HEADER_SIGNATURE: u64 = 0x5452415020494645;
    const HEADER_SIGNATURE_STR: &[u8] = b"EFI PART";
}
