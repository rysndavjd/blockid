pub mod aix;
pub mod gpt;
pub mod mbr;

use uuid::Uuid;

use crate::{
    error::Error,
    io::{BlockIo, Reader},
    partition::mbr::{MBR_MAGICS, probe_mbr},
    probe::{Id, Magic},
};

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
    fn pt_handler<IO: BlockIo>(&self) -> PtHandler<IO> {
        match self {
            PTType::Mbr => PtHandler {
                minsz: None,
                magics: MBR_MAGICS,
                probe: probe_mbr,
            },
            _ => todo!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartType {
    Hex(u8),
    Uuid(Uuid),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartId {
    Uuid(Uuid),
    Mbr { disk: u32, partno: u8 },
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartTableTag {
    PtType(PTType),
    PtId(Id),
    // EntryScheme(String),
    PartName(String),
    PartId(PartId),
    PartType(PartType),
    PartFlags(u64),
    PartNumber(u64),
    PartOffset(u64),
    PartSize(u64),
}

#[derive(Debug)]
pub struct PartTableInfo {
    tags: Vec<PartTableTag>,
}
