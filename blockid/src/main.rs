use std::fs::File;

use libblockid::*;

use rustix::fs::major;
use rustix::fs::minor;
use rustix::fs::Dev;
use uuid::Uuid;
use byteorder::LittleEndian;
use byteorder::ByteOrder;

fn test() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("/dev/sdb2")?; 
    let mut probe = BlockidProbe::new(&file, 0, 0, ProbeFlags::empty(), ProbeFilter::empty())?;
    

    let magic =            BlockidMagic {
            magic: b"SWAPSPACE2",
            len: 10,
            b_offset: 0xff6,
        };

    //let result = probe_swap_v1(&mut probe, magic)?;
    
   // println!("{:?}", probe);

    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{:?}", e),
    };
    
}
