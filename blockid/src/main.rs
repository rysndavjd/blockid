use std::fs::File;
use std::str::from_utf8;

use byteorder::BigEndian;
use libblockid::containers::luks::probe_luks2;
use libblockid::filesystems::exfat::probe_exfat;
use libblockid::partitions::dos::probe_dos_pt;
use libblockid::*;

use rustix::fs::major;
use rustix::fs::minor;
use rustix::fs::Dev;
use uuid::Uuid;
use byteorder::LittleEndian;
use byteorder::ByteOrder;
use libblockid::filesystems::vfat::*;

fn test() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("/dev/nvme0n1p6")?; 
    let mut probe = BlockidProbe::new(&file, 0, 0, ProbeFlags::empty(), ProbeFilter::empty())?;
    
    pub const LUKS2_MAGIC: [u8; 6] = *b"SKUL\xba\xbe";

    let magic =    BlockidMagic {
            magic: &LUKS2_MAGIC,
            len: 6,
            b_offset: 0,
    };

    let result = probe_luks2(&mut probe, magic)?;
    
    println!("{:?}", probe);

    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{:?}", e),
    };
    
}
