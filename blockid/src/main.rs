use std::fs::File;
use std::os::fd::AsFd;

use libblockid::filesystems::ntfs::{probe_ntfs, NTFS_ID_INFO};
use libblockid::*;

use byteorder::ByteOrder;
use byteorder::LittleEndian;
use rustix::fs::Dev;
use rustix::fs::major;
use rustix::fs::minor;
use uuid::Uuid;

fn test() -> Result<(), Box<dyn std::error::Error>> {
    //let file = File::open("/dev/sdb")?;

    let mut result = BlockidProbe::probe_from_filename("/dev/sdb3", ProbeFlags::empty(), ProbeFilter::empty(), 0)?;

    //result.probe_values()?;
    match probe_ntfs(&mut result, NTFS_ID_INFO.magics[0]) {
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
