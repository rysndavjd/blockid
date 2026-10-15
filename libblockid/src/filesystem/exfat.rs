use bstr::BString;
use fat_volume_id::id32::VolumeId32;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, Unaligned, byteorder::LittleEndian, byteorder::U16,
    byteorder::U32, byteorder::U64, transmute_ref,
};

use crate::{
    error::Error,
    filesystem::{FsInfo, FsType},
    io::{BlockIo, Reader},
    probe::Magic,
    std::fmt,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ExFatError {
    HeaderChecksumInvalid,
    ProbablyDOS,
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
    Overflow,
}

impl fmt::Display for ExFatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExFatError::HeaderChecksumInvalid => write!(f, "Invalid header checksum"),
            ExFatError::ProbablyDOS => write!(f, "Filesystem looks like DOS/MBR"),
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
            ExFatError::Overflow => {
                write!(f, "An operation has produced a overflow")
            }
        }
    }
}

impl<E: fmt::Debug> From<ExFatError> for Error<E> {
    fn from(e: ExFatError) -> Self {
        Self::ExFat(e)
    }
}

pub const EXFAT_MINSZ: Option<u64> = Some(4194304);
pub const EXFAT_MAGICS: Option<&'static [Magic]> = Some(&[Magic {
    bytes: b"EXFAT   ",
    offset: 3,
}]);

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
struct ExFatSuperBlock {
    bootjmp: [u8; 3],
    fs_name: [u8; 8],
    must_be_zero: [u8; 53],
    partition_offset: U64<LittleEndian>,
    volume_length: U64<LittleEndian>,
    fat_offset: U32<LittleEndian>,
    fat_length: U32<LittleEndian>,
    clustor_heap_offset: U32<LittleEndian>,
    clustor_count: U32<LittleEndian>,
    first_clustor_of_root: U32<LittleEndian>,
    volume_serial: [u8; 4],
    vermin: u8,
    vermaj: u8,
    volume_flags: U16<LittleEndian>,
    bytes_per_sector_shift: u8,
    sectors_per_cluster_shift: u8,
    number_of_fats: u8,
    drive_select: u8,
    percent_in_use: u8,
    reserved: [u8; 7],
    boot_code: [u8; 390],
    boot_signature: U16<LittleEndian>,
}

// These functions should not overflow assuming that `valid_exfat()`
// checks are strict enough to reject invaild headers
impl ExFatSuperBlock {
    const FIRST_DATA_CLUSTER: u32 = 2;
    const LAST_DATA_CLUSTER: u32 = 0x0FFFFFF6;
    const ENTRY_SIZE: usize = 32;
    const ENTRY_EOD: u8 = 0x00;
    const ENTRY_LABEL: u8 = 0x83;
    const MAX_DIR_SIZE: u32 = 256 * 1024 * 1024;

    fn block_size(&self) -> usize {
        1usize << self.bytes_per_sector_shift
    }

    fn cluster_size(&self) -> usize {
        self.block_size() << self.sectors_per_cluster_shift
    }

    fn block_to_offset(&self, block: u64) -> u64 {
        block << self.bytes_per_sector_shift
    }

    fn cluster_to_block(&self, cluster: u32) -> u64 {
        u64::from(self.clustor_heap_offset)
            + ((u64::from(cluster) - u64::from(Self::FIRST_DATA_CLUSTER))
                << self.sectors_per_cluster_shift)
    }

    fn cluster_to_offset(&self, cluster: u32) -> u64 {
        self.block_to_offset(self.cluster_to_block(cluster))
    }

    fn next_cluster<IO: BlockIo>(
        &self,
        reader: &mut Reader<IO>,
        cluster: u32,
    ) -> Result<u32, Error<IO::Error>> {
        let fat_offset = self.block_to_offset(u64::from(self.fat_offset)) + (cluster as u64 * 4);
        let next: [u8; 4] = reader.read_exact_at(fat_offset)?;

        return Ok(u32::from_le_bytes(next));
    }

    fn verify_checksum<IO: BlockIo>(
        &self,
        reader: &mut Reader<IO>,
        offset: u64,
    ) -> Result<(), Error<IO::Error>> {
        let sector_size = self.block_size();
        let sectors = reader.read_vec_at(offset, sector_size * 12)?;

        let checksum = {
            let n_bytes = sector_size * 11;

            let mut checksum: u32 = 0;
            for (i, byte) in sectors.iter().enumerate().take(n_bytes) {
                if i == 106 || i == 107 || i == 112 {
                    continue;
                }

                checksum = checksum.rotate_right(1).wrapping_add(*byte as u32);
            }
            checksum
        };

        let checksum_sector = &sectors[sector_size * 11..sector_size * 12];

        for bytes in checksum_sector.as_chunks::<4>().0 {
            let expected = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);

            if checksum != expected {
                return Err(ExFatError::HeaderChecksumInvalid.into());
            }
        }

        return Ok(());
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

#[inline]
fn in_range_inclusive<T: PartialOrd>(val: T, start: T, stop: T) -> bool {
    val >= start && val <= stop
}

fn valid_exfat<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    sb: &ExFatSuperBlock,
) -> Result<(), Error<IO::Error>> {
    if u16::from(sb.boot_signature) != 0xAA55 {
        return Err(ExFatError::ProbablyDOS.into());
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
        // Does not need checked subtraction, due to check above on `bytes_per_sector_shift`
        25 - sb.bytes_per_sector_shift,
    ) {
        return Err(ExFatError::InvalidRangeOfSectorsPerClusterShift.into());
    }

    let total_fat_sectors = sb
        .fat_length
        .get()
        .checked_mul(u32::from(sb.number_of_fats))
        .ok_or(ExFatError::Overflow)?;

    if !in_range_inclusive(
        sb.fat_offset.get(),
        24,
        sb.clustor_heap_offset
            .get()
            .checked_sub(total_fat_sectors)
            .ok_or(ExFatError::Overflow)?,
    ) {
        return Err(ExFatError::InvalidRangeOfFatOffset.into());
    }

    if !in_range_inclusive(
        sb.clustor_heap_offset.get(),
        sb.fat_offset
            .get()
            .checked_add(total_fat_sectors)
            .ok_or(ExFatError::Overflow)?,
        u32::MAX,
    ) {
        return Err(ExFatError::InvalidRangeOfClustorHeapOffset.into());
    }

    if !in_range_inclusive(
        sb.first_clustor_of_root.get(),
        2,
        sb.clustor_count
            .get()
            .checked_add(1)
            .ok_or(ExFatError::Overflow)?,
    ) {
        return Err(ExFatError::InvalidRangeOfFirstClustorOfRoot.into());
    }

    if sb.cluster_size() == 0 {
        return Err(ExFatError::InvalidClusterSize.into());
    }

    sb.verify_checksum(reader, offset)?;

    return Ok(());
}

pub(crate) fn probe_is_exfat<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
) -> Result<bool, Error<IO::Error>> {
    let buf: [u8; size_of::<ExFatSuperBlock>()] = reader.read_exact_at(offset)?;
    let sb: &ExFatSuperBlock = transmute_ref!(&buf);

    if reader
        .get_magic(EXFAT_MAGICS.expect("EXFAT magics is not `None`"))?
        .is_none()
    {
        return Ok(false);
    }

    match valid_exfat(reader, offset, sb) {
        Ok(_) => return Ok(true),
        Err(_) => return Ok(false),
    }
}

fn find_label<IO: BlockIo>(
    reader: &mut Reader<IO>,
    sb: &ExFatSuperBlock,
) -> Result<Option<BString>, Error<IO::Error>> {
    let mut cluster = sb.first_clustor_of_root.get();
    let mut offset = sb.cluster_to_offset(cluster);

    let mut i = 0;
    let mut buf: [u8; ExFatSuperBlock::ENTRY_SIZE] = [0u8; 32];

    while i < (ExFatSuperBlock::MAX_DIR_SIZE / (ExFatSuperBlock::ENTRY_SIZE as u32)) {
        reader.read_at(offset, &mut buf)?;
        let entry: &ExfatEntryLabel = transmute_ref!(&buf);

        if entry.label_type == ExFatSuperBlock::ENTRY_EOD {
            return Ok(None);
        }
        if entry.label_type == ExFatSuperBlock::ENTRY_LABEL {
            if entry.name == [0u8; 22] {
                return Ok(None);
            }

            return Ok(Some(BString::from(entry.name)));
        }

        offset += ExFatSuperBlock::ENTRY_SIZE as u64;

        if sb.cluster_size() != 0 && offset.is_multiple_of(sb.cluster_size() as u64) {
            cluster = sb.next_cluster(reader, cluster)?;
            if cluster < ExFatSuperBlock::FIRST_DATA_CLUSTER {
                return Ok(None);
            }
            if cluster > ExFatSuperBlock::LAST_DATA_CLUSTER {
                return Ok(None);
            }
            offset = sb.cluster_to_offset(cluster);
        }
        i += 1;
    }

    Ok(None)
}

pub(crate) fn probe_exfat<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    magic: Magic,
) -> Result<FsInfo, Error<IO::Error>> {
    let buf: [u8; size_of::<ExFatSuperBlock>()] = reader.read_exact_at(offset)?;
    let sb: &ExFatSuperBlock = transmute_ref!(&buf);

    valid_exfat(reader, offset, sb)?;

    let mut info = FsInfo::empty();

    info.set_fs_type(FsType::Exfat);
    info.set_fs_id(VolumeId32::from_bytes(sb.volume_serial).into());
    if let Some(l) = find_label(reader, sb)? {
        info.set_label(l.into());
    }
    info.set_fs_size(sb.block_size() as u64 * u64::from(sb.volume_length));
    info.set_fs_block_size(sb.block_size() as u64);
    info.set_block_size(sb.block_size() as u64);
    info.set_version(format!("{}.{}", sb.vermaj, sb.vermin));
    info.set_magic(magic.bytes.to_vec(), magic.offset);

    return Ok(info);
}
