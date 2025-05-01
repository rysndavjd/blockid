use rs_libblkid::mbr::*;
use uuid::Uuid;
use std::{ptr::read, str::from_utf8};

fn ext4_into() -> Result<(), Box<dyn std::error::Error>> {
    let device = read_generic_mbr("/dev/sdb")?;
    println!("{}", device.partition_entry_4.number_of_sectors);
    return Ok(());
}

fn main() {
    match ext4_into() {
        Ok(t) => t,
        Err(e) => eprintln!("{}", e),
    };
}
