use crate::{BlockidProbe, BlockidIdinfo, BlockidMagic, UsageType, ProbeResult};

const SOLARIS_SECTOR: u64 = 1;
const SOLARIS_OFFSET: u64 = SOLARIS_SECTOR << 9;
const SOLARIS_MAGICOFFSET: u64 = SOLARIS_OFFSET + 12;

pub const SOLARIS_X86_PT_IDINFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("solaris"),
    usage: Some(UsageType::PartitionTable),
    probe_fn: probe_solaris_pt,
    minsz: None,
    magics: &[
        BlockidMagic {
            magic: b"\xEE\xDE\x0D\x60",
            len: 4,
            b_offset: SOLARIS_MAGICOFFSET,
        },
    ]
};

fn probe_solaris_pt(
        probe: &mut BlockidProbe,
        mag: BlockidMagic,
    ) -> Result<Option<ProbeResult> ,Box<dyn std::error::Error>> 
{
    Ok(None)
}