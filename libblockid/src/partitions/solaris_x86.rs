use crate::probe::{BlockId, BlockMagic, Usage};

const SOLARIS_SECTOR: u64 = 1;
const SOLARIS_OFFSET: u64 = SOLARIS_SECTOR << 9;
const SOLARIS_MAGICOFFSET: u64 = SOLARIS_OFFSET + 12;

pub const SOLARIS_X86_PT_IDINFO: BlockId = BlockId {
    name: "solaris",
    usage: Some(Usage::PartTable),
    minsz: None,
    magics: &[
        BlockMagic {
            magic: b"\xEE\xDE\x0D\x60",
            len: 4,
            b_offset: SOLARIS_MAGICOFFSET,
        },
    ]
};