use std::fs::File;
use std::os::fd::AsFd;

use libblockid::filesystems::ntfs::{probe_ntfs, NTFS_ID_INFO};
use libblockid::partitions::bsd::{probe_bsd_pt, BSD_PT_IDINFO};
use libblockid::*;

use byteorder::ByteOrder;
use byteorder::LittleEndian;
use rustix::fs::Dev;
use rustix::fs::major;
use rustix::fs::minor;
use uuid::Uuid;

fn test() -> Result<(), Box<dyn std::error::Error>> {
    //let file = File::open("/dev/sdb")?;

    let mut result = BlockidProbe::probe_from_filename("/dev/sdb", ProbeFlags::empty(), ProbeFilter::empty(), 0)?;

    //result.probe_values()?;
    match probe_bsd_pt(&mut result, BSD_PT_IDINFO.magics[1]) {
        Ok(_) => println!("Ok"),
        Err(e) => println!("{}", e),
    }

    println!("{:?}", result);
    
    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{:?}", e),
    };
}
