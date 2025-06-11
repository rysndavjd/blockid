use crc::{Crc, Algorithm};

const CRC32C: Algorithm<u32> = Algorithm {
    width: 32,
    poly: 0x1EDC6F41,
    init: 0xFFFFFFFF,
    refin: true,
    refout: true,
    xorout: 0xFFFFFFFF,
    check: 0xE3069283,
    residue: 0xB798B438,
};

pub fn verify_crc32c(
        bytes: &[u8],
        checksum: u32,
    ) -> bool
{
    let crc = crc::Crc::<u32>::new(&CRC32C);
    let mut digest = crc.digest();
    digest.update(bytes);

    return digest.finalize() == checksum;
}

pub fn get_crc32c(
        bytes: &[u8],
    ) -> u32
{
    let crc = Crc::<u32>::new(&CRC32C);
    let mut digest = crc.digest();
    digest.update(bytes);

    return digest.finalize();
}