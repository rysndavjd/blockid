use std::fs::File;
use std::str::from_utf8;

use byteorder::BigEndian;
use libblockid::vfat::*;
use libblockid::*;

use uuid::Uuid;
use byteorder::LittleEndian;
use byteorder::ByteOrder;

fn test() -> Result<(), Box<dyn std::error::Error>> {
    //let vs = read_as_vfat("/home/rysndavjd/github/blockid/sample-headers/fat12.bin")?;
    //let ms = read_as_msdos("/home/rysndavjd/github/blockid/sample-headers/fat12.bin")?;
    //
    //let device = fat_type(vs, ms)?;
    let device = probe_is_vfat(&File::open("/home/rysndavjd/github/blockid/sample-headers/fat16.bin")?)?;
    println!("{:?}", device);

    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{}", e),
    };
    
}
