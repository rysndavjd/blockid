use crc::{Crc, CRC_32_ISCSI};

pub fn verify_crc32c(
        bytes: &[u8; 4],
        checksum: u32,
    ) -> bool
{
    let crc = crc::Crc::<u32>::new(&CRC_32_ISCSI);
    let mut digest = crc.digest();
    digest.update(bytes);

    return digest.finalize() == checksum;
}

pub fn get_crc32c(
        bytes: &[u8; 4],
    ) -> u32
{
    let crc = Crc::<u32>::new(&CRC_32_ISCSI);
    let mut digest = crc.digest();
    digest.update(bytes);

    return digest.finalize();
}