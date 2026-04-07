use fat_volume_id::VolumeId32;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, Unaligned, byteorder::LittleEndian, byteorder::U16,
    byteorder::U32, byteorder::U64, transmute,
};

use crate::{
    error::{Error, ErrorKind},
    io::{BlockIo, Reader},
    probe::{BlockInfo, BlockType, Endianness, Id, Magic, Tag, Usage},
    std::fmt,
    util::{decode_utf16_lossy_from},
};

#[derive(Debug)]
pub enum ExFatError {
    HeaderChecksumInvalid,
    ProbablyDOS,
    ProbablyNotEXFAT,
    InvalidClusterSize,
    InvalidBootJump,
    InvalidFsName,
    InvalidMustBeZero,
    InvalidRangeNumberOfFats,
    InvalidRangeOfBytesPerSectorShift,
    InvalidRangeOfSectorsPerClusterShift,
    InvalidRangeOfFatOffset,
    InvalidRangeOfClustorHeapOffset,
    InvalidRangeOfFirstClustorOfRoot,
}

impl fmt::Display for ExFatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExFatError::HeaderChecksumInvalid => write!(f, "Invalid header checksum"),
            ExFatError::ProbablyDOS => write!(f, "Filesystem looks like DOS/MBR"),
            ExFatError::ProbablyNotEXFAT => write!(f, "Filesystem does not look like EXFAT"),
            ExFatError::InvalidClusterSize => write!(f, "Invalid cluster size"),
            ExFatError::InvalidBootJump => write!(f, "Invalid boot jump"),
            ExFatError::InvalidFsName => write!(f, "Invalid filesystem name"),
            ExFatError::InvalidMustBeZero => write!(f, "Invalid must_be_zero field"),
            ExFatError::InvalidRangeNumberOfFats => write!(f, "Invalid range of number of fats"),
            ExFatError::InvalidRangeOfBytesPerSectorShift => {
                write!(f, "Invalid range of bytes per sector shift")
            }
            ExFatError::InvalidRangeOfSectorsPerClusterShift => {
                write!(f, "Invalid range of sectors per cluster shift")
            }
            ExFatError::InvalidRangeOfFatOffset => write!(f, "Invalid range of fat offset"),
            ExFatError::InvalidRangeOfClustorHeapOffset => {
                write!(f, "Invalid range of clustor heap offset")
            }
            ExFatError::InvalidRangeOfFirstClustorOfRoot => {
                write!(f, "Invalid range of first clustor of root")
            }
        }
    }
}

impl<IO: BlockIo> From<ExFatError> for Error<IO> {
    fn from(e: ExFatError) -> Self {
        Self(ErrorKind::ExFatError(e))
    }
}

pub const EXFAT_MAGICS: Option<&'static [Magic]> = Some(&[Magic {
    magic: b"EXFAT   ",
    len: 8,
    b_offset: 3,
}]);

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct ExFatSuperBlock {
    pub bootjmp: [u8; 3],
    pub fs_name: [u8; 8],
    must_be_zero: [u8; 53],
    pub partition_offset: U64<LittleEndian>,
    pub volume_length: U64<LittleEndian>,
    pub fat_offset: U32<LittleEndian>,
    pub fat_length: U32<LittleEndian>,
    pub clustor_heap_offset: U32<LittleEndian>,
    pub clustor_count: U32<LittleEndian>,
    pub first_clustor_of_root: U32<LittleEndian>,
    pub volume_serial: [u8; 4],
    pub vermin: u8,
    pub vermaj: u8,
    pub volume_flags: U16<LittleEndian>,
    pub bytes_per_sector_shift: u8,
    pub sectors_per_cluster_shift: u8,
    pub number_of_fats: u8,
    pub drive_select: u8,
    pub percent_in_use: u8,
    reserved: [u8; 7],
    pub boot_code: [u8; 390],
    pub boot_signature: U16<LittleEndian>,
}

impl ExFatSuperBlock {
    fn block_size(&self) -> usize {
        if self.bytes_per_sector_shift < 32 {
            1usize << self.bytes_per_sector_shift
        } else {
            0
        }
    }

    fn cluster_size(&self) -> usize {
        if self.sectors_per_cluster_shift < 32 {
            self.block_size() << self.sectors_per_cluster_shift
        } else {
            0
        }
    }

    fn block_to_offset(&self, block: u64) -> u64 {
        return block << self.bytes_per_sector_shift;
    }

    fn cluster_to_block(&self, cluster: u32) -> u64 {
        return u64::from(self.clustor_heap_offset)
            + (((cluster - EXFAT_FIRST_DATA_CLUSTER) as u64) << self.sectors_per_cluster_shift);
    }

    fn cluster_to_offset(&self, cluster: u32) -> u64 {
        return self.block_to_offset(self.cluster_to_block(cluster));
    }

    fn next_cluster<IO: BlockIo>(
        &self,
        reader: &mut Reader<IO>,
        cluster: u32,
    ) -> Result<u32, Error<IO>> {
        let fat_offset = self.block_to_offset(u64::from(self.fat_offset)) + (cluster as u64 * 4);
        let next: [u8; 4] = reader.read_exact_at(fat_offset).map_err(Error::io)?;

        return Ok(u32::from_le_bytes(next));
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
struct ExfatEntryLabel {
    label_type: u8,
    length: u8,
    name: [u8; 22],
    reserved: [u8; 8],
}

const EXFAT_FIRST_DATA_CLUSTER: u32 = 2;
const EXFAT_LAST_DATA_CLUSTER: u32 = 0x0FFFFFF6;
const EXFAT_ENTRY_SIZE: usize = 32;

const EXFAT_ENTRY_EOD: u8 = 0x00;
const EXFAT_ENTRY_LABEL: u8 = 0x83;

// 256 * 1024 * 1024
//const EXFAT_MAX_DIR_SIZE: u32 = 268435456;

pub fn get_exfatcsum(sectors: &[u8], sector_size: usize) -> u32 {
    let n_bytes = sector_size * 11;

    let mut checksum: u32 = 0;

    for (i, byte) in sectors.iter().enumerate().take(n_bytes) {
        if i == 106 || i == 107 || i == 112 {
            continue;
        }

        checksum = checksum.rotate_right(1).wrapping_add(*byte as u32);
    }

    return checksum;
}

fn verify_exfat_checksum<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    sb: ExFatSuperBlock,
) -> Result<(), Error<IO>> {
    let sector_size = sb.block_size();
    let data = reader
        .read_vec_at(offset, sector_size * 12)
        .map_err(Error::io)?;
    let checksum = get_exfatcsum(&data, sector_size);

    for i in 0..(sector_size / 4) {
        let offset = sector_size * 11 + i * 4;
        if let Some(bytes) = data.get(offset..offset + 4) {
            let expected = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);

            if checksum != expected {
                return Err(ExFatError::HeaderChecksumInvalid.into());
            }
        } else {
            return Err(ExFatError::HeaderChecksumInvalid.into());
        }
    }

    return Ok(());
}

#[inline]
fn in_range_inclusive<T: PartialOrd>(val: T, start: T, stop: T) -> bool {
    val >= start && val <= stop
}

fn valid_exfat<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    sb: ExFatSuperBlock,
) -> Result<(), Error<IO>> {
    if u16::from(sb.boot_signature) != 0xAA55 {
        return Err(ExFatError::ProbablyDOS.into());
    }

    if sb.cluster_size() == 0 {
        return Err(ExFatError::InvalidClusterSize.into());
    }

    if sb.bootjmp != [0xEB, 0x76, 0x90] {
        return Err(ExFatError::InvalidBootJump.into());
    }

    if &sb.fs_name != b"EXFAT   " {
        return Err(ExFatError::InvalidFsName.into());
    }

    if sb.must_be_zero != [0u8; 53] {
        return Err(ExFatError::InvalidMustBeZero.into());
    }

    if !in_range_inclusive(sb.number_of_fats, 1, 2) {
        return Err(ExFatError::InvalidRangeNumberOfFats.into());
    }

    if !in_range_inclusive(sb.bytes_per_sector_shift, 9, 12) {
        return Err(ExFatError::InvalidRangeOfBytesPerSectorShift.into());
    }

    if !in_range_inclusive(
        sb.sectors_per_cluster_shift,
        0,
        25 - sb.bytes_per_sector_shift,
    ) {
        return Err(ExFatError::InvalidRangeOfSectorsPerClusterShift.into());
    }

    if !in_range_inclusive(
        u32::from(sb.fat_offset),
        24,
        u32::from(sb.clustor_heap_offset) - (u32::from(sb.fat_length) * sb.number_of_fats as u32),
    ) {
        return Err(ExFatError::InvalidRangeOfFatOffset.into());
    }

    if !in_range_inclusive(
        u32::from(sb.clustor_heap_offset),
        u32::from(sb.fat_offset) + u32::from(sb.fat_length) * sb.number_of_fats as u32,
        1u32 << (32 - 1),
    ) {
        return Err(ExFatError::InvalidRangeOfClustorHeapOffset.into());
    }

    if !in_range_inclusive(
        u32::from(sb.first_clustor_of_root),
        2,
        u32::from(sb.clustor_count) + 1,
    ) {
        return Err(ExFatError::InvalidRangeOfFirstClustorOfRoot.into());
    }

    verify_exfat_checksum(reader, offset, sb)?;

    return Ok(());
}

// pub fn probe_is_exfat<IO: BlockIo>(reader: &mut Reader<IO>) -> Result<(), Error<IO>> {
//     let sb: ExFatSuperBlock =
//         probe.map_from_file::<{ size_of::<ExFatSuperBlock>() }, ExFatSuperBlock>(probe.offset())?;

//     if probe.get_magic(&EXFAT_ID_INFO).is_ok() {
//         return Err(ExFatError::ProbablyNotEXFAT);
//     }

//     valid_exfat(probe, sb)?;

//     return Ok(());
// }

fn find_label<IO: BlockIo>(
    reader: &mut Reader<IO>,
    sb: ExFatSuperBlock,
) -> Result<Option<String>, Error<IO>> {
    let mut cluster = u32::from(sb.first_clustor_of_root);
    let mut offset = sb.cluster_to_offset(cluster);

    let mut i = 0;

    while i < 8388608 {
        // EXFAT_MAX_DIR_SIZE / EXFAT_ENTRY_SIZE
        let buf = match reader.read_exact_at::<EXFAT_ENTRY_SIZE>(offset) {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };

        let entry: ExfatEntryLabel = transmute!(buf);

        if entry.label_type == EXFAT_ENTRY_EOD {
            return Ok(None);
        }
        if entry.label_type == EXFAT_ENTRY_LABEL {
            if entry.name == [0u8; 22] {
                return Ok(None);
            }
            let label = decode_utf16_lossy_from(&entry.name, Endianness::Little);
            return Ok(Some(label.to_string()));
        }

        offset += EXFAT_ENTRY_SIZE as u64;

        if sb.cluster_size() != 0 && offset.is_multiple_of(sb.cluster_size() as u64) {
            cluster = sb.next_cluster(reader, cluster)?;
            if cluster < EXFAT_FIRST_DATA_CLUSTER {
                return Ok(None);
            }
            if cluster > EXFAT_LAST_DATA_CLUSTER {
                return Ok(None);
            }
            offset = sb.cluster_to_offset(cluster);
        }
        i += 1;
    }

    Ok(None)
}

pub fn probe_exfat<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    mag: Magic,
) -> Result<BlockInfo, Error<IO>> {
    let buf: [u8; size_of::<ExFatSuperBlock>()] =
        reader.read_exact_at(offset).map_err(Error::io)?;

    let sb = transmute!(buf);

    valid_exfat(reader, offset, sb)?;

    let label = find_label(reader, sb)?;

    let version = sb.vermaj.to_string() + "." + &sb.vermin.to_string();

    let mut info = BlockInfo::new();

    info.set(Tag::FsType(BlockType::Exfat));
    info.set(Tag::Id(Id::VolumeId32(VolumeId32::from_bytes(
        sb.volume_serial,
    ))));
    if let Some(l) = label {
        info.set(Tag::Label(l));
    }
    info.set(Tag::Usage(Usage::Filesystem));
    info.set(Tag::FsSize(
        sb.block_size() as u64 * u64::from(sb.volume_length),
    ));
    info.set(Tag::FsBlockSize(sb.block_size() as u64));
    info.set(Tag::BlockSize(sb.block_size() as u64));
    info.set(Tag::Usage(Usage::Filesystem));
    info.set(Tag::Version(version));
    info.set(Tag::Magic(mag.magic.to_vec()));
    info.set(Tag::MagicOffset(mag.b_offset));

    return Ok(info);
}
