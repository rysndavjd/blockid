#[derive(Debug)]
pub enum CsumAlgorium {
    Crc32(u64),
    Crc32c(u64),
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