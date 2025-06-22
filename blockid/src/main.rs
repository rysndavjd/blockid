use std::fs::File;
use std::str::from_utf8;

use byteorder::BigEndian;
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
    let file = File::open("/dev/sdb")?; 
    let mut probe = BlockidProbe::new(&file, 0, 0, ProbeFlags::empty(), ProbeFilter::empty())?;

    let magic = BlockidMagic {
        magic: b"\x55\xAA",
        len: 2,
        b_offset: 510,
    };

    let result = probe_dos_pt(&mut probe, magic)?;
    
    println!("{:?}", result);

    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{:?}", e),
    };
    
}
