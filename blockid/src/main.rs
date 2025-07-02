use std::fs::File;

use libblockid::filesystems::ntfs::probe_ntfs;
use libblockid::*;

use byteorder::ByteOrder;
use byteorder::LittleEndian;
use rustix::fs::Dev;
use rustix::fs::major;
use rustix::fs::minor;
use uuid::Uuid;

fn test() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("/dev/sdb3")?;
    let mut probe = BlockidProbe::new(&file, 0, 0, ProbeFlags::empty(), ProbeFilter::empty())?;

    let magic =         BlockidMagic {
            magic: b"NTFS    ",
            len: 8,
            b_offset: 3,
        };

    let result = probe_ntfs(&mut probe, magic)?;

    println!("{:X?}", probe);

    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{:?}", e),
    };
}
