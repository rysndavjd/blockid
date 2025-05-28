use uuid::Uuid;
use std::fs::File;
use std::path::Path;
use bitflags::bitflags;
use bytemuck::{from_bytes, Pod};
use std::io::{Read, Seek, SeekFrom};
use rustix::fs::{stat, Stat};

use crate::filesystems::vfat::{VfatExtras, VfatVersion, probe_vfat, VFAT_ID_INFO};
use crate::filesystems::volume_id::{self, VolumeId32, VolumeId64};

pub fn get_buffer(
        probe: &mut BlockProbe,
        offset: u64,
        buffer_size: usize,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> 
{
    let mut block = probe.file.try_clone()?;
    block.seek(SeekFrom::Start(0))?;

    let mut buffer = vec![0u8; buffer_size];
    block.seek(SeekFrom::Start(offset))?;
    block.read_exact(&mut buffer)?;

    return Ok(buffer);
}

pub fn get_sector(
        probe: &mut BlockProbe,
        sector: u64,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> 
{
    get_buffer(probe, sector << 9, 0x200)
}

pub fn probe_get_magic(
        probe: &mut BlockProbe, 
        id_info: &BlockId
    ) -> Result<BlockMagic, Box<dyn std::error::Error>>
{
    for magic in id_info.magics {
        let b_offset: u64 = magic.b_offset;
        let magic_len: usize = magic.len.try_into().unwrap(); // FIX

        let mut raw = probe.file.try_clone()?;
        raw.seek(SeekFrom::Start(b_offset))?;

        let mut buffer = vec![0; magic_len];

        raw.read_exact(&mut buffer)?;

        if buffer == magic.magic {
            return Ok(magic.clone());
        }
    }
    return Err("Unable to find any magic".into());
}

pub fn read_as<T: Pod>(
        raw_block: &File,
        offset: u64,
    ) -> Result<T, Box<dyn std::error::Error>> 
{
    let mut block = raw_block.try_clone()?;
    block.seek(SeekFrom::Start(0))?;

    let mut buffer = vec![0u8; std::mem::size_of::<T>()];
    block.seek(SeekFrom::Start(offset))?;
    block.read_exact(&mut buffer)?;

    let ptr = from_bytes::<T>(&buffer);
    Ok(*ptr)
}

pub fn get_dev_t<P: AsRef<Path>>(path: P) -> Option<u64> {
    let stat: Stat = stat(path.as_ref()).ok()?;
    Some(stat.st_rdev) 
}

pub fn get_disk_devno<P: AsRef<Path>>(path: P) -> Option<u64> {
    let stat: Stat = stat(path.as_ref()).ok()?;
    Some(stat.st_dev) 
}

fn probe_from_filename(filename: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(filename)?;
    
    //let probe = BlockProbe::new(file, 0, 0, Stat::from(2), disk_devno)

    return Ok(());
}