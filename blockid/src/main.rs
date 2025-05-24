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

    let ProbeResults: ProbeResults = ProbeResults{
        fs_type: None,
        sec_type: None,
        uuid: None,
        uuid_sub: None,
        label: None,
        label_raw: None,
        fs_version: None,
        usage: None,
        part_uuid: None,
        part_name: None,
        part_number: None,
        part_scheme: None
    };

    let mut probe: BlockProbe = BlockProbe { file: File::open("/dev/sdb3")?,
                                        begin: 0, end: 0, devno: 0, disk_devno: 0, probe_flags: BlkidFlags::empty(), values: ProbeResults};

    let magic = probe_get_magic(&mut probe, &VFAT_ID_INFO)?;
    let device = probe_vfat(&mut probe, magic)?;
    //println!("{:?}", device);

    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{}", e),
    };
    
}
