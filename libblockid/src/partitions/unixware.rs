use crate::probe::{BlockId, BlockMagic, Usage};

const UNIXWARE_SECTOR: u64 = 29;
const UNIXWARE_OFFSET: u64 = UNIXWARE_SECTOR << 9;
const UNIXWARE_KBOFFSET: u64 = UNIXWARE_OFFSET >> 10;
const UNIXWARE_MAGICOFFSET: u64 = UNIXWARE_OFFSET - UNIXWARE_KBOFFSET + 4;

pub const UNIXWARE_PT_IDINFO: BlockId = BlockId {
    name: "unixware",
    usage: Some(Usage::PartTable),
    minsz: Some(1024*1440+1),
    magics: &[
        BlockMagic {
            magic: b"\x0D\x60\xE5\xCA",
            len: 4,
            b_offset: UNIXWARE_MAGICOFFSET,
        }
    ]
};
