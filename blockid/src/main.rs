use std::fs::File;
use std::str::from_utf8;

use byteorder::BigEndian;
use libblockid::probe::BlockProbe;
use libblockid::*;
use libblockid::probe::*;

use uuid::Uuid;
use byteorder::LittleEndian;
use byteorder::ByteOrder;
use rustix::fs::{major, minor};


fn test() -> Result<(), Box<dyn std::error::Error>> {
    
    let dev_t = get_dev_t("/dev/sdb3").ok_or("eh")?;
    let test = get_disk_devno("/dev/sdb3").ok_or("eh")?;
    let major = major(dev_t);
    let minor = minor(dev_t);

    println!("{}", dev_t);
    println!("{}", major);
    println!("{}", minor);
    println!("{}", test);

    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{}", e),
    };
    
}
