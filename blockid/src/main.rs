use std::fs::File;
use std::str::from_utf8;

use byteorder::BigEndian;
use libblockid::*;

use uuid::Uuid;
use byteorder::LittleEndian;
use byteorder::ByteOrder;
use crc::{Algorithm, Crc};

const CRC32C: Algorithm<u32> = Algorithm {
    width: 32,
    poly: 0x1EDC6F41,
    init: 0xFFFFFFFF,
    refin: true,
    refout: true,
    xorout: 0xFFFFFFFF,
    check: 0xE3069283,
    residue: 0xB798B438,
};

fn test() -> Result<(), Box<dyn std::error::Error>> {
    
    let crc = crc::Crc::<u32>::new(&CRC32C);
    let mut digest = crc.digest();
    digest.update(b"1");

    println!("{:X}", digest.finalize());
    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{}", e),
    };
    
}
