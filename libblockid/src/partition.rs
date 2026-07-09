pub(crate) mod aix;
pub(crate) mod bsd;
pub(crate) mod gpt;
pub(crate) mod mbr;

use bitflags::bitflags;
use uuid::Uuid;

use crate::{
    error::Error,
    io::{BlockIo, Reader},
    partition::{
        aix::{AIX_MAGICS, AIX_MINSZ, probe_aix},
        gpt::{GPT_MAGICS, GPT_MINSZ, probe_gpt},
        mbr::{MBR_MAGICS, MBR_MINSZ, MbrPartitionType, probe_mbr},
    },
    probe::{Magic, ProbeFlags},
    std::fmt,
};

/// Order used to detect partition tables
#[rustfmt::skip]
pub const PT_DETECT_ORDER: &[(PTFilter, PartTableType)] = &[
    (PTFilter::SKIP_GPT, PartTableType::Gpt),
    (PTFilter::SKIP_MBR, PartTableType::Mbr),
];

/// A generic handler for probing a partition table type.
#[derive(Debug, Copy, Clone)]
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
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PartTableType {
    /// AIX partition table is used on the [IBM AIX](https://en.wikipedia.org/wiki/IBM_AIX) operating system
    Aix,
    /// [Master boot record partition table](https://en.wikipedia.org/wiki/Master_boot_record).
    Mbr,
    /// [GUID Partition Table](https://en.wikipedia.org/wiki/GUID_Partition_Table).
    Gpt,
}

impl fmt::Display for PartTableType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PartTableType::Aix => write!(f, "aix"),
            PartTableType::Mbr => write!(f, "mbr"),
            PartTableType::Gpt => write!(f, "gpt"),
        }
    }
}

impl PartTableType {
    pub(crate) const fn pt_handler<IO: BlockIo>(&self) -> PtHandler<IO> {
        match self {
            PartTableType::Aix => PtHandler {
                minsz: AIX_MINSZ,
                magics: AIX_MAGICS,
                probe: probe_aix,
            },
            PartTableType::Mbr => PtHandler {
                minsz: MBR_MINSZ,
                magics: MBR_MAGICS,
                probe: probe_mbr,
            },
            PartTableType::Gpt => PtHandler {
                minsz: GPT_MINSZ,
                magics: GPT_MAGICS,
                probe: probe_gpt,
            },
        }
    }
}

/// Identifier used by a filesystem or partition table.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PartTableId {
    /// A 128-bit universally unique identifier.
    Uuid(Uuid),
    /// A 32-bit MBR disk signature.
    Mbr { disk: u32 },
}

impl PartTableId {
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            PartTableId::Uuid(t) => Some(*t),
            _ => None,
        }
    }

    pub fn as_mbr(&self) -> Option<u32> {
        match self {
            PartTableId::Mbr { disk } => Some(*disk),
            _ => None,
        }
    }
}

impl From<Uuid> for PartTableId {
    fn from(value: Uuid) -> Self {
        PartTableId::Uuid(value)
    }
}

impl From<u32> for PartTableId {
    fn from(disk: u32) -> Self {
        PartTableId::Mbr { disk }
    }
}

/// The partition type of a specified partition table.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PartitionType {
    /// [Partition types](https://en.wikipedia.org/wiki/Partition_type) used in MBR partition table.
    Mbr(MbrPartitionType),
    /// [Partition types GUIDs](https://en.wikipedia.org/wiki/GUID_Partition_Table#Partition_type_GUIDs) used in GPT partition table.
    Uuid(Uuid),
    /// Used for MAC partition table.
    String(String),
}

/// The partition identifier of a specified partition table.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PartitionId {
    /// Used for GPT and MAC partition tables.
    Uuid(Uuid),
    /// A pseudo partition identifier used for MBR partition table.
    Mbr { disk: u32, part_no: u8 },
}

impl PartitionId {
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            PartitionId::Uuid(t) => Some(*t),
            _ => None,
        }
    }

    pub fn as_mbr(&self) -> Option<(u32, u8)> {
        match self {
            PartitionId::Mbr { disk, part_no } => Some((*disk, *part_no)),
            _ => None,
        }
    }
}

/// The partition attributes of a specified partition table.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PartAttributes {
    /// Used in MBR partition tables for if partition is active or inactive.
    Mbr(u8),
    /// Used in GPT partition tables.
    Gpt(u64),
}

/// Parsed partition infomation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Partition {
    /// Start of partition in bytes.
    pub start: u64,
    /// End of partition in bytes.
    pub end: u64,
    /// The partition identifier of a specified partition table.
    pub partition_id: PartitionId,
    /// The partition type of a specified partition table.
    pub partition_type: PartitionType,
    /// Partition number, starting from 1
    pub part_no: u64,
    /// Partition label
    pub partition_name: Option<String>,
    /// The partition attributes of a specified partition table.
    pub attributes: PartAttributes,
}

#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PartTableTag {
    /// Partition table type.
    PartTableType(PartTableType),
    /// Partition table identifier.
    PartTableId(PartTableId),
    /// Total size in bytes from the start of the disk to the end of the
    /// partition table addressed region.
    PartTableSize(u64),
    /// Partition table magic signature.
    Magic(Vec<u8>),
    /// Partition table magic signature offset.
    MagicOffset(u64),
    /// List of partitions in the partition table.
    Partitions(Vec<Partition>),
}

#[derive(Debug)]
#[repr(transparent)]
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

    pub fn part_table_type(&self) -> Option<PartTableType> {
        self.tags.iter().find_map(|t| match t {
            PartTableTag::PartTableType(t) => Some(*t),
            _ => None,
        })
    }

    pub fn part_table_id(&self) -> Option<PartTableId> {
        self.tags.iter().find_map(|t| match t {
            PartTableTag::PartTableId(t) => Some(*t),
            _ => None,
        })
    }

    pub fn pt_size(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            PartTableTag::PartTableSize(t) => Some(*t),
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

#[cfg(feature = "serde")]
impl serde::Serialize for PartTableInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(self.tags.len()))?;

        for tag in &self.tags {
            match tag {
                PartTableTag::PartTableType(pt) => {
                    map.serialize_entry("PT_TYPE", pt)?;
                }
                PartTableTag::PartTableId(id) => match id {
                    PartTableId::Uuid(uuid) => map.serialize_entry("PT_ID", uuid)?,
                    PartTableId::Mbr { disk } => {
                        map.serialize_entry("PT_ID", &format!("{:x}", disk))?;
                    }
                },
                PartTableTag::PartTableSize(sz) => {
                    map.serialize_entry("PT_SIZE", sz)?;
                }
                PartTableTag::Magic(mag) => {
                    map.serialize_entry("PT_MAGIC", mag)?;
                }
                PartTableTag::MagicOffset(off) => {
                    map.serialize_entry("PT_MAGIC_OFFSET", off)?;
                }
                PartTableTag::Partitions(parts) => {
                    for part in parts {
                        map.serialize_entry(&format!("PART{}_START", part.part_no), &part.start)?;
                        map.serialize_entry(&format!("PART{}_END", part.part_no), &part.end)?;
                        match &part.partition_id {
                            PartitionId::Uuid(uuid) => {
                                map.serialize_entry(&format!("PART{}_ID", part.part_no), uuid)?;
                            }
                            PartitionId::Mbr { disk, part_no } => {
                                map.serialize_entry(
                                    &format!("PART{}_ID", part.part_no),
                                    &format!("{:#x}{:x}", disk, part_no),
                                )?;
                            }
                        }
                        match &part.partition_type {
                            PartitionType::Mbr(byte) => {
                                map.serialize_entry(&format!("PART{}_TYPE", part.part_no), byte)?;
                            }
                            PartitionType::Uuid(uuid) => {
                                map.serialize_entry(&format!("PART{}_TYPE", part.part_no), uuid)?;
                            }
                            PartitionType::String(str) => {
                                map.serialize_entry(&format!("PART{}_TYPE", part.part_no), str)?;
                            }
                        }
                        if let Some(name) = &part.partition_name {
                            map.serialize_entry(&format!("PART{}_NAME", part.part_no), name)?;
                        }
                        match &part.attributes {
                            PartAttributes::Mbr(attr) => {
                                map.serialize_entry(
                                    &format!("PART{}_ATTRIBUTES", part.part_no),
                                    attr,
                                )?;
                            }
                            PartAttributes::Gpt(attr) => {
                                map.serialize_entry(
                                    &format!("PART{}_ATTRIBUTES", part.part_no),
                                    attr,
                                )?;
                            }
                        }
                    }
                }
            }
        }

        map.end()
    }
}

bitflags! {
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct PTFilter: u64 {
        const SKIP_MBR = 1 << 0;
        const SKIP_GPT = 1 << 1;
    }
}
