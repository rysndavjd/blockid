use std::fs::File;
use std::str::from_utf8;

use byteorder::BigEndian;
use libblockid::*;

use rustix::fs::major;
use rustix::fs::minor;
use rustix::fs::Dev;
use uuid::Uuid;
use byteorder::LittleEndian;
use byteorder::ByteOrder;
use libblockid::filesystems::ext::*;

fn test() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("/dev/sdb3")?; 

    let mut probe = BlockidProbe::new(&file, 0, 0)?;

    let magic = BlockidMagic {
        magic: &[0x53, 0xEF],
        len: 2,
        b_offset: 0x38,
    };
    
    //let probe = probe_ext4(&mut probe, magic)?;

    //println!("{:?}", probe);
    for prob in PROBES {
        match (prob.probe_fn)(&mut probe, magic) {
            Ok(t) => println!("{:?}", t),
            Err(e) => eprintln!("{}", e),
        };
        
    }

    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{}", e),
    };
    
}
