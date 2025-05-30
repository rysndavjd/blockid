use crate::{BlockidProbe, BlockidIdinfo, BlockidMagic, Usage};

pub const MINIX_PT_IDINFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("minix"),
    usage: Some(Usage::PartitionTable),
    probe_fn: probe_minix_pt,
    minsz: None,
    magics: &[
        BlockidMagic {
            magic: b"\x55\xAA",
            len: 2,
            b_offset: 510,
        },
    ]
};

fn probe_minix_pt(
        probe: &mut BlockidProbe,
        mag: BlockidMagic,
    ) -> Result<() ,Box<dyn std::error::Error>> 
{
    Ok(())
}