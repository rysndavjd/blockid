use std::fs::File;
use std::str::from_utf8;

use byteorder::BigEndian;
use libblockid::*;
use libblockid::partitions::dos::*;

use uuid::Uuid;
use byteorder::LittleEndian;
use byteorder::ByteOrder;


fn test() -> Result<(), Box<dyn std::error::Error>> {
    
    let mut probe = BlockidProbe::new(File::open("/dev/sdb")?);

    probe_dos_pt(&mut probe, DOS_PT_ID_INFO.magics[0])?;

    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{}", e),
    };
    
}
