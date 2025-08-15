use std::{
    str::Utf8Error,
    fs::read_dir,
    io::{Error as IoError, ErrorKind, Seek, Read, SeekFrom},
    path::{Path, PathBuf},
};

use glob::{glob, GlobError};
use thiserror::Error;
use widestring::{utfstring::Utf16String, error::Utf16Error};
use rustix::fs::{stat, FileType, Dev};
use zerocopy::FromBytes;

use crate::{probe::{BlockidIdinfo, BlockidMagic, BlockidUUID, Endianness, ProbeResult}, BlockidError, Probe, ProbeFilter, ProbeFlags, ProbeMode};

#[derive(Debug, Error)]
pub enum UtfError {
    #[error("UTF-8 Error: {0}")]
    Utf8Error(#[from] Utf8Error),
    #[error("UTF-16 Error: {0}")]
    Utf16Error(#[from] Utf16Error),
}

pub fn decode_utf16_lossy_from(bytes: &[u8], endian: Endianness) -> Utf16String {
    let data: Vec<u16> = bytes
        .chunks(2)
        .filter_map(|chunk| {
            if chunk.len() == 2 {
                let val = match endian {
                    Endianness::Big => u16::from_be_bytes([chunk[0], chunk[1]]),
                    Endianness::Little => u16::from_le_bytes([chunk[0], chunk[1]]),
                };
                if val == 0 {
                    None
                } else {
                    Some(val)
                }
            } else {
                None
            }
        })
        .collect();

    return Utf16String::from_slice_lossy(&data).into();
}

pub fn decode_utf8_lossy_from(bytes: &[u8]) -> String {
    return String::from_utf8_lossy(bytes)
        .trim_end_matches('\0')
        .to_string();
}

pub fn decode_utf16_from(bytes: &[u8], endian: Endianness) -> Result<Utf16String, UtfError> {
    let data: Vec<u16> = bytes
        .chunks(2)
        .filter_map(|chunk| {
            if chunk.len() == 2 {
                let val = match endian {
                    Endianness::Big => u16::from_be_bytes([chunk[0], chunk[1]]),
                    Endianness::Little => u16::from_le_bytes([chunk[0], chunk[1]]),
                };
                if val == 0 {
                    None
                } else {
                    Some(val)
                }
            } else {
                None
            }
        })
        .collect();

    return Ok(Utf16String::from_vec(data)?);
}

pub fn decode_utf8_from(bytes: &[u8]) -> Result<String, UtfError> {
    return Ok(String::from_utf8(bytes.to_vec())
        .map_err(|e| e.utf8_error())?
        .trim_end_matches('\0')
        .to_string());
}

pub fn is_power_2(num: u64) -> bool {
    return num != 0 && ((num & (num - 1)) == 0); 
}

pub fn devno_to_path(dev: Dev) -> Result<PathBuf, IoError> {
    let dev_dir = read_dir(Path::new("/dev"))?;

    for entry in dev_dir.flatten() {
        let path = entry.path();

        if let Ok(stat) = stat(&path) {

            if FileType::from_raw_mode(stat.st_mode).is_block_device()
                && stat.st_rdev == dev 
            {
                return Ok(path);
            }
        }
    }
    return Err(IoError::new(ErrorKind::NotFound, "Unable to find path from devno"));
}

pub fn path_to_devno<P: AsRef<Path>>(path: P) -> Result<Dev, IoError> {
    let stat = stat(path.as_ref())?;
    if FileType::from_raw_mode(stat.st_mode).is_block_device() {
        return Ok(stat.st_rdev)
    } else {
        return Err(IoError::new(ErrorKind::InvalidInput, "Path doesnt point to a block device"));
    }
}

pub fn probe_get_magic<R: Read+Seek>(
        file: &mut R, 
        id_info: &BlockidIdinfo
    ) -> Result<Option<BlockidMagic>, IoError>
{
    match id_info.magics {
        Some(magics) => {
            for magic in magics {
                file.seek(SeekFrom::Start(magic.b_offset))?;

                let mut buffer = vec![0; magic.len];

                file.read_exact(&mut buffer)?;

                if buffer == magic.magic {
                    return Ok(Some(*magic));
                }
            }
        },
        None => {
            return Ok(None);
        },
    }

    return Err(ErrorKind::NotFound.into());
}

pub fn from_file<T: FromBytes, R: Read+Seek>(
        file: &mut R,
        offset: u64,
    ) -> Result<T, IoError> 
{
    let mut buffer = vec![0u8; core::mem::size_of::<T>()];
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut buffer)?;

    let data = T::read_from_bytes(&buffer)
        .map_err(|_| ErrorKind::UnexpectedEof)?;
    
    return Ok(data);
}

pub fn read_exact_at<const S: usize, R: Read+Seek>(
        file: &mut R,
        offset: u64,
    ) -> Result<[u8; S], IoError>
{
    let mut buffer = [0u8; S];
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut buffer)?;

    return Ok(buffer);
}

pub fn read_vec_at<R: Read+Seek>(
        file: &mut R,
        offset: u64,
        buf_size: usize
    ) -> Result<Vec<u8>, IoError>
{
    let mut buffer = vec![0u8; buf_size];
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut buffer)?;

    return Ok(buffer);
}

pub fn read_sector_at<R: Read+Seek>(
        file: &mut R,
        sector: u64,
    ) -> Result<[u8; 512], IoError>
{
    return read_exact_at::<512, R>(file, sector << 9);
}

pub fn device_from(uuid: &BlockidUUID) -> Result<PathBuf, BlockidError> {
    let patterns = [
        "/dev/sd*",      
        "/dev/hd*",      
        "/dev/nvme*n*",  
        "/dev/loop*",    
        "/dev/ram*",     
        "/dev/md*",      
        "/dev/mapper/*",
    ];

    for pattern in patterns {
        for entry in glob(pattern).expect("THIS SHOULD NOT FAIL") {
            let path = entry?;
            let stat = stat(&path)?; 
            
            let mut probe = Probe::from_filename(&path, ProbeMode::Single, ProbeFlags::empty(), ProbeFilter::empty(), 0)?;
            probe.probe_values()?;

            let value = match probe.result().ok_or(BlockidError::ProbeError("No device found"))? {
                ProbeResult::Container(r) => r.uuid,
                ProbeResult::PartTable(r) => r.uuid,
                ProbeResult::Filesystem(r) => r.uuid,
                ProbeResult::List(_) => None,
            };

            if FileType::from_raw_mode(stat.st_mode).is_block_device()
                && &value.ok_or(BlockidError::ProbeError("AHHH"))? == uuid
            {
                return Ok(path);
            }
        }
    }
    return Err(BlockidError::ResultError("AHHH"));
}