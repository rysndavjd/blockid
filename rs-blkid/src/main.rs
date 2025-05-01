use rs_libblkid::mbr::*;
use uuid::Uuid;
use std::str::from_utf8;

fn ext4_into() -> Result<(), Box<dyn std::error::Error>> {
    let device = check_mbr("/dev/sdb")?;

    return Ok(());
}

fn main() {
    match ext4_into() {
        Ok(t) => t,
        Err(e) => eprintln!("{}", e),
    };
}
