use crate::probe::{BlockId, BlockMagic, Usage};

pub const MINIX_PT_IDINFO: BlockId = BlockId {
    name: "minix",
    usage: Some(Usage::PartTable),
    minsz: None,
    magics: &[
        BlockMagic {
            magic: b"\x55\xAA",
            len: 2,
            b_offset: 510,
        },
    ]
};