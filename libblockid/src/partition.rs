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
        gpt::{GPT_MAGICS, GPT_MINSZ, GptAttributes, probe_gpt},
        mbr::{MBR_MAGICS, MBR_MINSZ, MbrAttributes, MbrPartitionType, probe_mbr},
    },
    probe::{Label, Magic},
    std::fmt,
};

/// Order used to detect partition tables
#[rustfmt::skip]
pub const PT_DETECT_ORDER: &[(PtFilter, PtType)] = &[
    (PtFilter::SKIP_GPT, PtType::Gpt),
    (PtFilter::SKIP_MBR, PtType::Mbr),
];

/// A generic handler for probing a partition table type.
#[derive(Debug, Copy, Clone)]
pub(crate) struct PtHandler<IO: BlockIo> {
    /// Minimum disk size in bytes required for partition table, if any.
    pub minsz: Option<u64>,
    /// Magic signatures used to identify filesystem, if any.
    pub magics: Option<&'static [Magic]>,
    /// Probes the partition table, returning its info on success.
    #[allow(clippy::type_complexity)]
    pub probe: fn(&mut Reader<IO>, u64, Magic) -> Result<PtInfo, Error<IO::Error>>,
}

/// The type of partition tables supported.
#[non_exhaustive]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PtType {
    /// AIX partition table is used on the [IBM AIX](https://en.wikipedia.org/wiki/IBM_AIX) operating system
    Aix,
    /// [Master boot record partition table](https://en.wikipedia.org/wiki/Master_boot_record).
    Mbr,
    /// [GUID Partition Table](https://en.wikipedia.org/wiki/GUID_Partition_Table).
    Gpt,
}

impl fmt::Display for PtType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PtType::Aix => write!(f, "aix"),
            PtType::Mbr => write!(f, "mbr"),
            PtType::Gpt => write!(f, "gpt"),
        }
    }
}

impl PtType {
    pub(crate) const fn pt_handler<IO: BlockIo>(&self) -> PtHandler<IO> {
        match self {
            PtType::Aix => PtHandler {
                minsz: AIX_MINSZ,
                magics: AIX_MAGICS,
                probe: probe_aix,
            },
            PtType::Mbr => PtHandler {
                minsz: MBR_MINSZ,
                magics: MBR_MAGICS,
                probe: probe_mbr,
            },
            PtType::Gpt => PtHandler {
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
pub enum PtId {
    /// A 128-bit universally unique identifier or [UUID](https://en.wikipedia.org/wiki/Universally_unique_identifier).
    Uuid(Uuid),
    /// A 32-bit MBR disk signature.
    Mbr { disk: u32 },
}

impl PtId {
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            PtId::Uuid(t) => Some(*t),
            _ => None,
        }
    }

    pub fn as_mbr(&self) -> Option<u32> {
        match self {
            PtId::Mbr { disk } => Some(*disk),
            _ => None,
        }
    }
}

impl From<Uuid> for PtId {
    fn from(value: Uuid) -> Self {
        PtId::Uuid(value)
    }
}

impl From<u32> for PtId {
    fn from(disk: u32) -> Self {
        PtId::Mbr { disk }
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

    /// Currently we return the disk ID and the partition number, eventully I
    /// will probally make a custom mbr type or something like fat_volume_id
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
pub enum PartitionAttributes {
    Mbr(MbrAttributes),
    Gpt(GptAttributes),
}

/// Parsed partition infomation.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone)]
pub struct Partition {
    /// Partition number, starting from 1
    pub part_no: u64,
    /// Start of partition in bytes.
    pub start: u64,
    /// End of partition in bytes.
    pub end: u64,
    /// The partition identifier of a specified partition table.
    pub partition_id: PartitionId,
    /// The partition type of a specified partition table.
    pub partition_type: PartitionType,
    /// Partition label
    pub partition_name: Option<Label>,
    /// The partition attributes of a specified partition table.
    pub attributes: PartitionAttributes,
}

#[derive(Debug)]
pub struct PtInfo {
    /// Partition table type.
    pt_type: Option<PtType>,
    /// Partition table identifier.
    pt_id: Option<PtId>,
    /// Total size in bytes from the start of the disk to the end of the
    /// partition table addressed region.
    pt_size: Option<u64>,
    /// Partition table magic bytes.
    magic: Option<Vec<u8>>,
    /// Partition table magic offset.
    magic_offset: Option<u64>,
    /// List of partitions in the partition table.
    partitions: Option<Vec<Partition>>,
}

impl PtInfo {
    pub(crate) fn empty() -> PtInfo {
        PtInfo {
            pt_type: None,
            pt_id: None,
            pt_size: None,
            magic: None,
            magic_offset: None,
            partitions: None,
        }
    }

    pub(crate) const fn set_pt_type(&mut self, pt_type: PtType) {
        self.pt_type = Some(pt_type)
    }

    pub fn pt_type(&self) -> Option<PtType> {
        self.pt_type
    }

    pub(crate) const fn set_pt_id(&mut self, pt_id: PtId) {
        self.pt_id = Some(pt_id)
    }

    pub fn pt_id(&self) -> Option<PtId> {
        self.pt_id
    }

    pub(crate) const fn set_pt_size(&mut self, pt_size: u64) {
        self.pt_size = Some(pt_size)
    }

    pub fn pt_size(&self) -> Option<u64> {
        self.pt_size
    }

    pub(crate) fn set_magic(&mut self, bytes: Vec<u8>, offset: u64) {
        self.magic = Some(bytes);
        self.magic_offset = Some(offset);
    }

    pub fn magic(&self) -> Option<(&[u8], u64)> {
        match (&self.magic, &self.magic_offset) {
            (Some(magic), Some(offset)) => Some((magic.as_slice(), *offset)),
            (None, None) => None,
            _ => unreachable!("magic and magic_offset are only ever set together via `set_magic`"),
        }
    }

    pub(crate) fn set_partitions(&mut self, partitions: Vec<Partition>) {
        self.partitions = Some(partitions)
    }

    pub fn partitions(&self) -> Option<&[Partition]> {
        self.partitions.as_deref()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for PtInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let len = self.pt_type.is_some() as usize
            | self.pt_id.is_some() as usize
            | self.pt_size.is_some() as usize
            | if self.magic.is_some() && self.magic_offset.is_some() {
                2
            } else {
                0
            }
            | self.partitions.is_some() as usize;

        let mut map = serializer.serialize_map(Some(len))?;

        if let Some(v) = &self.pt_type {
            map.serialize_entry("PT_TYPE", v)?;
        }
        if let Some(v) = &self.pt_id {
            match v {
                PtId::Uuid(uuid) => map.serialize_entry("PT_ID", uuid)?,
                PtId::Mbr { disk } => {
                    map.serialize_entry("PT_ID", &format!("{:x}", disk))?;
                }
            }
        }
        if let Some(v) = &self.pt_size {
            map.serialize_entry("PT_SIZE", v)?;
        }
        match (&self.magic, &self.magic_offset) {
            (Some(magic), Some(offset)) => {
                map.serialize_entry("MAGIC", magic)?;
                map.serialize_entry("MAGIC_OFFSET", offset)?;
            }
            (None, None) => {}
            _ => unreachable!("magic and magic_offset are only ever set together via set_magic"),
        }
        if let Some(v) = &self.partitions {
            for part in v {
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
                    PartitionAttributes::Mbr(attr) => {
                        map.serialize_entry(&format!("PART{}_ATTRIBUTES", part.part_no), attr)?;
                    }
                    PartitionAttributes::Gpt(attr) => {
                        map.serialize_entry(&format!("PART{}_ATTRIBUTES", part.part_no), attr)?;
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
    pub struct PtFilter: u64 {
        #[bitflags(flag_name = "aix")]
        const SKIP_AIX = 1 << 0;
        #[bitflags(flag_name = "mbr")]
        const SKIP_MBR = 1 << 1;
        #[bitflags(flag_name = "gpt")]
        const SKIP_GPT = 1 << 2;
    }
}
