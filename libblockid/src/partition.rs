pub(crate) mod aix;
pub(crate) mod gpt;
pub(crate) mod mbr;

use bitflags::bitflags;
use uuid::Uuid;

use crate::{
    error::Error,
    io::{BlockIo, Reader},
    partition::{
        gpt::{GPT_MAGICS, GPT_MINSZ, probe_gpt},
        mbr::{MBR_MAGICS, MBR_MINSZ, probe_mbr},
    },
    probe::{Id, Magic, ProbeFlags},
};

/// Order used to detect partition tables in [`probe_part_table`]
/// 
/// [`probe_part_table`]: crate::probe::Probe::probe_part_table
#[rustfmt::skip]
pub const PT_DETECT_ORDER: &[(PTFilter, PTType)] = &[
    (PTFilter::SKIP_GPT, PTType::Gpt),
];

/// A generic handler for probing a partition table type.
#[derive(Debug, Copy, Clone, Hash)]
pub(crate) struct PtHandler<IO: BlockIo> {
    /// Minimum disk size in bytes required for partition table, if any.
    pub minsz: Option<u64>,
    /// Minimum disk size in bytes required for this partition table, if any.
    pub magics: Option<&'static [Magic]>,
    /// Probes the partition table, returning its info on success.
    #[allow(clippy::type_complexity)]
    pub probe:
        fn(&mut Reader<IO>, ProbeFlags, u64, Magic) -> Result<PartTableInfo, Error<IO::Error>>,
}

/// The type of partition tables supported.
#[non_exhaustive]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PTType {
    /// [Master boot record partition table](https://en.wikipedia.org/wiki/Master_boot_record).
    Mbr,
    /// [GUID Partition Table](https://en.wikipedia.org/wiki/GUID_Partition_Table).
    Gpt,
}

impl PTType {
    pub(crate) fn pt_handler<IO: BlockIo>(&self) -> PtHandler<IO> {
        match self {
            PTType::Mbr => PtHandler {
                minsz: MBR_MINSZ,
                magics: MBR_MAGICS,
                probe: probe_mbr,
            },
            PTType::Gpt => PtHandler {
                minsz: GPT_MINSZ,
                magics: GPT_MAGICS,
                probe: probe_gpt,
            },
        }
    }
}

/// The partition type of a specified partition table.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PartType {
    /// [Partition types](https://en.wikipedia.org/wiki/Partition_type) used in MBR partition table.
    Hex(u8),
    /// [Partition types GUIDs](https://en.wikipedia.org/wiki/GUID_Partition_Table#Partition_type_GUIDs) used in GPT partition table.
    Uuid(Uuid),
    /// Used for MAC partition table.
    String(String),
}

/// The partition identifier of a specified partition table.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PartId {
    /// Used for GPT and MAC partition tables.
    Uuid(Uuid),
    /// A pseudo partition identifier used for MBR partition table.
    Mbr { disk: u32, part_no: u8 },
}

/// The partition attributes of a specified partition table.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PartAttributes {
    /// Used in MBR partition tables for if partition is active or inactive.
    Mbr(u8),
    /// Used in GPT partition tables.
    Gpt(u64),
}

/// Parsed partition infomation.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Partition {
    /// Start of partition in bytes.
    start: u64,
    /// End of partition in bytes.
    end: u64,
    /// The partition identifier of a specified partition table.
    partition_id: PartId,
    /// The partition type of a specified partition table.
    partition_type: PartType,
    /// Partition number
    part_no: u64,
    /// Partition label
    partition_name: Option<String>,
    /// The partition attributes of a specified partition table.
    attributes: PartAttributes,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PartTableTag {
    PtType(PTType),
    PtId(Id),
    PtSize(u64),
    Magic(Vec<u8>),
    MagicOffset(u64),
    Partitions(Vec<Partition>),
}

#[derive(Debug)]
pub struct PartTableInfo {
    tags: Vec<PartTableTag>,
}

impl PartTableInfo {
    pub(crate) fn new() -> PartTableInfo {
        PartTableInfo { tags: Vec::new() }
    }

    pub fn inner(&self) -> &[PartTableTag] {
        self.tags.as_slice()
    }

    pub fn into_inner(self) -> Vec<PartTableTag> {
        self.tags
    }

    pub(crate) fn set(&mut self, tag: PartTableTag) {
        self.tags.push(tag);
    }

    pub fn pt_type(&self) -> Option<PTType> {
        self.tags.iter().find_map(|t| match t {
            PartTableTag::PtType(t) => Some(*t),
            _ => None,
        })
    }

    pub fn id(&self) -> Option<Id> {
        self.tags.iter().find_map(|t| match t {
            PartTableTag::PtId(t) => Some(*t),
            _ => None,
        })
    }

    pub fn pt_size(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            PartTableTag::PtSize(t) => Some(*t),
            _ => None,
        })
    }

    pub fn magic(&self) -> Option<&[u8]> {
        self.tags.iter().find_map(|t| match t {
            PartTableTag::Magic(t) => Some(t.as_slice()),
            _ => None,
        })
    }

    pub fn magic_offset(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            PartTableTag::MagicOffset(t) => Some(*t),
            _ => None,
        })
    }

    pub fn partitions(&self) -> Option<&[Partition]> {
        self.tags.iter().find_map(|t| match t {
            PartTableTag::Partitions(t) => Some(t.as_slice()),
            _ => None,
        })
    }
}

bitflags! {
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct PTFilter: u64 {
        const SKIP_MBR = 1 << 0;
        const SKIP_GPT = 1 << 1;
    }
}
