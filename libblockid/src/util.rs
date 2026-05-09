use widestring::{error::Utf16Error, utfstring::Utf16String};

use crate::{probe::Endianness, std::str::Utf8Error};

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

pub fn decode_utf16_from(bytes: &[u8], endian: Endianness) -> Result<Utf16String, Utf16Error> {
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

pub fn decode_utf8_from(bytes: &[u8]) -> Result<String, Utf8Error> {
    return Ok(String::from_utf8(bytes.to_vec())
        .map_err(|e| e.utf8_error())?
        .trim_end_matches('\0')
        .to_string());
}

pub fn fletcher64(buf: &[u8]) -> u64 {
    let mut lo32: u64 = 0;
    let mut hi32: u64 = 0;

    for i in 0..(buf.len() / 4) {
        let offset = i * 4;
        let word = u32::from_le_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]) as u64;
        lo32 = lo32.wrapping_add(word);
        hi32 = hi32.wrapping_add(lo32);
    }

    let csum_lo = !((lo32.wrapping_add(hi32)) % 0xFFFFFFFF) as u32;
    let csum_hi = !((lo32.wrapping_add(csum_lo as u64)) % 0xFFFFFFFF) as u32;

    return ((csum_hi as u64) << 32) | (csum_lo as u64);
}
