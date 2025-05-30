use crate::{BlockidProbe, BlockidIdinfo, BlockidMagic, Usage};

pub const BSD_PT_IDINFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("bsd"),
    usage: Some(Usage::PartitionTable),
    probe_fn: probe_bsd_pt,
    minsz: None,
    magics: &[
        BlockidMagic {
            magic: b"\x57\x45\x56\x82",
            len: 4,
            b_offset: 512,
        },
        BlockidMagic {
            magic: b"\x57\x45\x56\x82",
            len: 4,
            b_offset: 64,
        },
        BlockidMagic {
            magic: b"\x57\x45\x56\x82",
            len: 4,
            b_offset: 128,
        },
    ]
};

fn probe_bsd_pt(
        probe: &mut BlockidProbe,
        mag: BlockidMagic,
    ) -> Result<() ,Box<dyn std::error::Error>> 
{
    Ok(())
}