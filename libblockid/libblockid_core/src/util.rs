use widestring::{error::Utf16Error, utfstring::Utf16String};

use crate::probe::Endianness;
use crate::std::{fmt, str::Utf8Error};

#[derive(Debug)]
pub enum UtfError {
    Utf8Error(Utf8Error),
    Utf16Error(Utf16Error),
}

impl From<Utf8Error> for UtfError {
    fn from(e: Utf8Error) -> Self {
        UtfError::Utf8Error(e)
    }
}

impl From<Utf16Error> for UtfError {
    fn from(e: Utf16Error) -> Self {
        UtfError::Utf16Error(e)
    }
}

impl crate::std::error::Error for UtfError {}

impl fmt::Display for UtfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Utf8Error(e) => write!(f, "UTF-8 error occurred: {e}"),
            Self::Utf16Error(e) => write!(f, "UTF-16 error occurred: {e}"),
        }
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
                if val == 0 { None } else { Some(val) }
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
                if val == 0 { None } else { Some(val) }
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
