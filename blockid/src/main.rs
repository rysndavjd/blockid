use std::fs::File;
use std::str::from_utf8;

use byteorder::BigEndian;
use libblockid::probe::BlockProbe;
use libblockid::vfat::*;
use libblockid::volume_id::VolumeId64;
use libblockid::*;
use libblockid::probe::*;

use uuid::Uuid;
use byteorder::LittleEndian;
use byteorder::ByteOrder;

fn test() -> Result<(), Box<dyn std::error::Error>> {
    
    let file = File::open("/dev/sdb3")?;

    let mut probe = BlockProbe::new(file, 0, 0, 0, 0);
    let magic = probe_get_magic(&mut probe, &VFAT_ID_INFO)?;
    
    //let fat_size: u32 = get_fat_size(ms, vs);
    //let sector_size: u32 = ms.ms_sector_size.into();
    //let reserved: u32 = ms.ms_reserved.into();

    //let root_start: u32 = (reserved + fat_size) * sector_size;
    //let root_dir_entries: u32 = vs.vs_dir_entries.into();

    //let device = search_fat_label(&mut probe, root_start, root_dir_entries)?;
    let id = VolumeId64::from_u64_le(2013176438932066366);

    println!("{}", id);
    println!("{:X}", id);
    println!("{:x}", id);
    println!("{:o}", id);
    println!("{:b}", id);
    //let device = probe_vfat(&mut probe, magic)?;
    //println!("Probe: {:?}", probe);

    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{}", e),
    };
    
}
