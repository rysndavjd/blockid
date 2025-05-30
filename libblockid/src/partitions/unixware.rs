use crate::{BlockidProbe, BlockidIdinfo, BlockidMagic, Usage};

const UNIXWARE_SECTOR: u64 = 29;
const UNIXWARE_OFFSET: u64 = UNIXWARE_SECTOR << 9;
const UNIXWARE_KBOFFSET: u64 = UNIXWARE_OFFSET >> 10;
const UNIXWARE_MAGICOFFSET: u64 = UNIXWARE_OFFSET - UNIXWARE_KBOFFSET + 4;

pub const UNIXWARE_PT_IDINFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("unixware"),
    usage: Some(Usage::PartitionTable),
    probe_fn: probe_unixware_pt,
    minsz: Some(1024*1440+1),
    magics: &[
        BlockidMagic {
            magic: b"\x0D\x60\xE5\xCA",
            len: 4,
            b_offset: UNIXWARE_MAGICOFFSET,
        }
    ]
};

fn probe_unixware_pt(
        probe: &mut BlockidProbe,
        mag: BlockidMagic,
    ) -> Result<() ,Box<dyn std::error::Error>> 
{
    Ok(())
}