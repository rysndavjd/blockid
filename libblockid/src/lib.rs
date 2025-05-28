pub mod partitions;
pub mod filesystems;
pub mod probe;

use std::fs::File;
use bitflags::bitflags;
use rustix::fs::{Dev, Mode};

bitflags! {
    struct BlockidFlags: u32 {
        const BLKID_FL_PRIVATE_FD = 1 << 1;
        const BLKID_FL_TINY_DEV = 1 << 2;
        const BLKID_FL_CDROM_DEV = 1 << 3;
        const BLKID_FL_NOSCAN_DEV = 1 << 4;
        const BLKID_FL_MODIF_BUFF = 1 << 5;
        const BLKID_FL_OPAL_LOCKED = 1 << 6;
        const BLKID_FL_OPAL_CHECKED = 1 << 7;
    }
}

bitflags! {
    struct BlockidProbFlags: u32 {
        const BLKID_PROBE_FL_IGNORE_PT = 1 << 1;
    }
}

struct BlockidProbe {
    file: File,
    begin: u64,
    end: u64,
    io_size: u64,

    devno: Dev,
    disk_devno: Dev,
    sector_size: u64,
    mode: Mode,
    zone_size: u64,

    flags: BlockidFlags,
    prob_flags: BlockidProbFlags
}



bitflags! {
    struct UsageFlags: u32 {
        const FILESYSTEM    = 1 << 1;
        const RAID          = 1 << 2;
        const CRYPTO        = 1 << 3;
        const OTHER         = 1 << 4;
    }
}

bitflags! {
    struct IdInfoFlags: u32 {
        const BLKID_IDINFO_TOLERANT    = 1 << 1;
    }
}

pub type FsProbeFn = fn(&mut BlockProbe, BlockMagic) -> Result<(), Box<dyn std::error::Error>>;

struct BlockidIdinfo {
    name: String,
    usage: UsageFlags,
    flags: IdInfoFlags,
    minsz: u64,

}