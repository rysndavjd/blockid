// Partition Tables
pub mod mbr;
pub mod gpt;

// Filesystems
pub mod ext4;
pub mod vfat;

// Library code
pub mod probe;
pub mod volume_id;

use crate::probe::*;

use uuid::Uuid;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

/*
ideas

make a probe function with a struct
that has basic info for filesystems and partitions eg like
struct filesystems
    file system uuid
    partition uuid 
    label
    filesystem type
    filesystem version 
    filesystem magic signature
    size of filesystem in bytes
    
struct partitions
    partition type
    partition table uuid/id
    partition name
    partition uuid
    partition number
    partition offset
    partition size
    disk maj:min
*/

fn is_power_2(num: u64) -> bool {
    return num != 0 && ((num & (num - 1)) == 0); 
}

pub fn read_raw(device: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut superblock = File::open(device)?;
    //superblock.seek(SeekFrom::Start(65536))?;
    let mut buffer = [0; 512];
    superblock.read_exact(&mut buffer)?;

    println!("{:X?}", buffer);
    return Ok(());

}
