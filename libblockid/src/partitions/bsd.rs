use crate::probe::{BlockId, BlockMagic, Usage};

pub const BSD_PT_IDINFO: BlockId = BlockId {
    name: "bsd",
    usage: Some(Usage::PartTable),
    minsz: None,
    magics: &[
        BlockMagic {
            magic: b"\x57\x45\x56\x82",
            len: 4,
            b_offset: 512,
        },
        BlockMagic {
            magic: b"\x57\x45\x56\x82",
            len: 4,
            b_offset: 64,
        },
        BlockMagic {
            magic: b"\x57\x45\x56\x82",
            len: 4,
            b_offset: 128,
        },
    ]
};