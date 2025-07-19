use crc::{Crc, CRC_32_ISO_HDLC, CRC_32_ISCSI};

#[derive(Debug)]
pub enum CsumAlgorium {
    Crc32(u32),
    Crc32c(u32),
    Exfat(u32),
    Ntfs(u32)
}

impl std::fmt::Display for CsumAlgorium {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CsumAlgorium::Crc32(checksum) => write!(f, "{checksum}"),
            CsumAlgorium::Crc32c(checksum) => write!(f, "{checksum}"),
            CsumAlgorium::Exfat(checksum) => write!(f, "{checksum}"),
            CsumAlgorium::Ntfs(checksum) => write!(f, "{checksum}"),
        }
    }
}

impl std::fmt::UpperHex for CsumAlgorium {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CsumAlgorium::Crc32(checksum) => write!(f, "{checksum:X}"),
            CsumAlgorium::Crc32c(checksum) => write!(f, "{checksum:X}"),
            CsumAlgorium::Exfat(checksum) => write!(f, "{checksum:X}"),
            CsumAlgorium::Ntfs(checksum) => write!(f, "{checksum:X}"),
        }
    }
}

impl std::fmt::LowerHex for CsumAlgorium {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CsumAlgorium::Crc32(checksum) => write!(f, "{checksum:x}"),
            CsumAlgorium::Crc32c(checksum) => write!(f, "{checksum:x}"),
            CsumAlgorium::Exfat(checksum) => write!(f, "{checksum:x}"),
            CsumAlgorium::Ntfs(checksum) => write!(f, "{checksum:x}"),
        }
    }
}

pub fn verify_crc32_iso_hdlc(
        bytes: &[u8],
        checksum: u32,
    ) -> bool
{
    return get_crc32_iso_hdlc(bytes) == checksum;
}

pub fn get_crc32_iso_hdlc(
        bytes: &[u8],
    ) -> u32
{
    let crc = Crc::<u32>::new(&CRC_32_ISO_HDLC);
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

