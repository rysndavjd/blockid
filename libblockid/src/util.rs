use std::str::Utf8Error;
use widestring::{utfstring::Utf16String, error::Utf16Error};
use crate::Endianness;

#[derive(Debug)]
pub enum UtfError {
    Utf8Error(Utf8Error),
    Utf16Error(Utf16Error),
}

impl std::fmt::Display for UtfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UtfError::Utf8Error(e) => write!(f, "UTF-8 Error: {e}"),
            UtfError::Utf16Error(e) => write!(f, "UTF-16 Error: {e}"),
        }
    }
}

impl From<Utf8Error> for UtfError {
    fn from(err: Utf8Error) -> Self {
        UtfError::Utf8Error(err)
    }
}

impl From<Utf16Error> for UtfError {
    fn from(err: Utf16Error) -> Self {
        UtfError::Utf16Error(err)
    }
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
