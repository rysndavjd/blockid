use std::fs::File;
use std::str::from_utf8;

use byteorder::BigEndian;
use libblockid::filesystems::exfat::probe_exfat;
use libblockid::*;

use rustix::fs::major;
use rustix::fs::minor;
use rustix::fs::Dev;
use uuid::Uuid;
use byteorder::LittleEndian;
use byteorder::ByteOrder;
use libblockid::filesystems::vfat::*;

fn test() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("/dev/sdb7")?; 
    let mut probe = BlockidProbe::new(&file, 0, 0, ProbeFlags::empty(), ProbeFilter::empty())?;

    let magic = BlockidMagic {
        magic: b"EXFAT   ",
        len: 8,
        b_offset: 3,
    };

    let result = probe_exfat(&mut probe, magic)?;
    
    println!("{:?}", result);

    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{:?}", e),
    };
    
}
