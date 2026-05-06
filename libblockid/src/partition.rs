pub mod aix;
pub mod gpt;
pub mod mbr;

use bitflags::bitflags;
use uuid::Uuid;

use crate::{
    error::Error,
    io::{BlockIo, Reader},
    partition::{
        gpt::{GPT_MAGICS, GPT_MINSZ, probe_gpt},
        mbr::{MBR_MAGICS, MBR_MINSZ, probe_mbr},
    },
    probe::{Id, Magic},
};

#[rustfmt::skip]
pub const PT_DETECT_ORDER: &[(PTFilter, PTType)] = &[
    (PTFilter::SKIP_GPT, PTType::Gpt),
];

#[derive(Debug, Copy, Clone, Hash)]
pub struct PtHandler<IO: BlockIo> {
    pub minsz: Option<u64>,
    pub magics: Option<&'static [Magic]>,
    #[allow(clippy::type_complexity)]
    pub probe: fn(&mut Reader<IO>, u64, Magic) -> Result<PartTableInfo, Error<IO::Error>>,
}

#[non_exhaustive]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PTType {
    Mbr,
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
            _ => todo!(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PartType {
    Hex(u8),
    Uuid(Uuid),
    String(String),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PartId {
    Uuid(Uuid),
    Mbr { disk: u32, part_no: u8 },
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PartAttributes {
    Mbr(u8),
    Gpt(u64),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Partition {
    start: u64,
    end: u64,
    partition_id: PartId,
    partition_type: PartType,
    part_no: u64,
    partition_name: Option<String>,
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
    Partions(Vec<Partition>),
}

#[derive(Debug)]
pub struct PartTableInfo {
    tags: Vec<PartTableTag>,
}

impl PartTableInfo {
    pub(crate) fn new() -> PartTableInfo {
        PartTableInfo { tags: Vec::new() }
    }

    pub fn inner(&self) -> &Vec<PartTableTag> {
        &self.tags
    }

    pub fn into_inner(self) -> Vec<PartTableTag> {
        self.tags
    }

    pub fn set(&mut self, tag: PartTableTag) {
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

    pub fn magic(&self) -> Option<&Vec<u8>> {
        self.tags.iter().find_map(|t| match t {
            PartTableTag::Magic(t) => Some(t),
            _ => None,
        })
    }

    pub fn magic_offset(&self) -> Option<u64> {
        self.tags.iter().find_map(|t| match t {
            PartTableTag::MagicOffset(t) => Some(*t),
            _ => None,
        })
    }

    pub fn partitions(&self) -> Option<&Vec<Partition>> {
        self.tags.iter().find_map(|t| match t {
            PartTableTag::Partions(t) => Some(t),
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
