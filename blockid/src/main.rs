use std::fs::File;
use std::os::fd::AsFd;

use libblockid::filesystems::ntfs::probe_ntfs;
use libblockid::ioctl::ioctl_blkgetsize64;
use libblockid::*;

use byteorder::ByteOrder;
use byteorder::LittleEndian;
use rustix::fs::Dev;
use rustix::fs::major;
use rustix::fs::minor;
use uuid::Uuid;

fn test() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("/dev/sdb3")?;

    let mut result = BlockidProbe::probe_from_filename("/dev/sdb3", ProbeFlags::empty(), ProbeFilter::empty(), 0)?;

    result.probe_values()?;

    println!("{:?}", result);

    //let value = ioctl_blkgetsize64(file.as_fd())?;

    //println!("{}", value);

    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{:?}", e),
    };
}
