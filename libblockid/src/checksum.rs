use core::fmt;
use crc::{Crc, CRC_32_CKSUM, CRC_32_ISCSI};

#[derive(Debug)]
pub enum CsumAlgorium {
    Crc32(u32),
    Crc32c(u32),
    Exfat(u32),
    NTFS(u32)
}

impl fmt::Display for CsumAlgorium {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CsumAlgorium::Crc32(checksum) => write!(f, "{}", checksum),
            CsumAlgorium::Crc32c(checksum) => write!(f, "{}", checksum),
            CsumAlgorium::Exfat(checksum) => write!(f, "{}", checksum),
            CsumAlgorium::NTFS(checksum) => write!(f, "{}", checksum),
        }
    }
}

impl fmt::UpperHex for CsumAlgorium {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CsumAlgorium::Crc32(checksum) => write!(f, "{:X}", checksum),
            CsumAlgorium::Crc32c(checksum) => write!(f, "{:X}", checksum),
            CsumAlgorium::Exfat(checksum) => write!(f, "{:X}", checksum),
            CsumAlgorium::NTFS(checksum) => write!(f, "{:X}", checksum),
        }
    }
}

impl fmt::LowerHex for CsumAlgorium {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CsumAlgorium::Crc32(checksum) => write!(f, "{:x}", checksum),
            CsumAlgorium::Crc32c(checksum) => write!(f, "{:x}", checksum),
            CsumAlgorium::Exfat(checksum) => write!(f, "{:x}", checksum),
            CsumAlgorium::NTFS(checksum) => write!(f, "{:x}", checksum),
        }
    }
}

pub fn verify_crc32(
        bytes: &[u8],
        checksum: u32,
    ) -> bool
{
    return get_crc32(bytes) == checksum;
}

pub fn get_crc32(
        bytes: &[u8],
    ) -> u32
{
    let crc = Crc::<u32>::new(&CRC_32_CKSUM);
    let mut digest = crc.digest();
    digest.update(bytes);

    return digest.finalize();
}

pub fn verify_crc32c(
        bytes: &[u8],
        checksum: u32,
    ) -> bool
{
    return get_crc32c(bytes) == checksum;
}

pub fn get_crc32c(
        bytes: &[u8],
    ) -> u32
{
    let crc = Crc::<u32>::new(&CRC_32_ISCSI);
    let mut digest = crc.digest();
    digest.update(bytes);

    return digest.finalize();
}

