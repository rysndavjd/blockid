use core::{str::Utf8Error, fmt::{self, Debug}};

use widestring::{utfstring::Utf16String, error::Utf16Error};
use crate::Endianness;

#[derive(Debug)]
pub enum UtfError {
    Utf8Error(Utf8Error),
    Utf16Error(Utf16Error),
}

impl fmt::Display for UtfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
                Some(match endian {
                    Endianness::Big => u16::from_be_bytes([chunk[0], chunk[1]]),
                    Endianness::Little => u16::from_le_bytes([chunk[0], chunk[1]]),
                })
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

pub fn is_power_2(num: u64) -> bool {
    return num != 0 && ((num & (num - 1)) == 0); 
}
