use std::str::from_utf8;

use byteorder::BigEndian;
use rs_libblkid::*;
use uuid::Uuid;
use byteorder::LittleEndian;
use byteorder::ByteOrder;

fn test() -> Result<(), Box<dyn std::error::Error>> {

    let device = read_raw("/dev/sdb6")?;

    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{}", e),
    };


    
    
}
