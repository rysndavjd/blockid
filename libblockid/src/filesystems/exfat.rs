use std::{io::{Error as IoError, Read, Seek}};

use zerocopy::{FromBytes, IntoBytes, Unaligned, 
    byteorder::U64, byteorder::U32, byteorder::U16, 
    byteorder::LittleEndian, Immutable, transmute};
use rustix::fs::makedev;

use crate::{
    checksum::CsumAlgorium, filesystems::{vfat::VFAT_ID_INFO, 
    volume_id::VolumeId32, FsError}, probe::{BlockType, BlockidIdinfo, 
    BlockidMagic, Probe, BlockidUUID, BlockidVersion, Endianness, 
    FilesystemResult, ProbeResult, UsageType}, util::{decode_utf16_lossy_from, 
    from_file, probe_get_magic, read_exact_at, read_vec_at, UtfError}, BlockidError
};

#[derive(Debug)]
pub enum ExFatError {
    IoError(IoError),
    UnknownFilesystem(&'static str),
    ExfatHeaderError(&'static str),
    UtfError(UtfError),
    ChecksumError {
        expected: CsumAlgorium,
        got: CsumAlgorium,
    }
}

impl std::fmt::Display for ExFatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExFatError::IoError(e) => write!(f, "I/O operation failed: {e}"),
            ExFatError::ExfatHeaderError(e) => write!(f, "Not an Exfat superblock: {e}"),
            ExFatError::UnknownFilesystem(e) => write!(f, "Exfat header error: {e}"),
            ExFatError::UtfError(e) => write!(f, "Unable to get UTF-16 string: {e}"),
            ExFatError::ChecksumError{expected, got} => write!(f, "Exfat Checksum failed, expected: \"{expected:X}\" and got: \"{got:X})\""),
        }
    }
}

impl From<ExFatError> for FsError {
    fn from(err: ExFatError) -> Self {
        match err {
            ExFatError::IoError(e) => FsError::IoError(e),
            ExFatError::ExfatHeaderError(info) => FsError::InvalidHeader(info),
            ExFatError::UtfError(_) => FsError::InvalidHeader("Invalid utf16 to convert to utf8"),
            ExFatError::UnknownFilesystem(info) => FsError::UnknownFilesystem(info),
            ExFatError::ChecksumError { expected, got } => FsError::ChecksumError { expected, got },
        }
    }
}

impl From<IoError> for ExFatError {
    fn from(err: IoError) -> Self {
        ExFatError::IoError(err)
    }
}

impl From<UtfError> for ExFatError {
    fn from(err: UtfError) -> Self {
        ExFatError::UtfError(err)
    }
}

pub const EXFAT_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("exfat"),
    btype: Some(BlockType::Exfat),
    usage: Some(UsageType::Filesystem),
    probe_fn: |probe, magic| {
        probe_exfat(probe, magic)
        .map_err(FsError::from)
        .map_err(BlockidError::from)
    },
    minsz: None,
    magics: Some(&[
        BlockidMagic {
            magic: b"EXFAT   ",
            len: 8,
            b_offset: 3,
        },
    ])
};

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
    fn block_size(
            &self
        ) -> usize 
    {
        if self.bytes_per_sector_shift < 32 {
            1usize << self.bytes_per_sector_shift
        } else {
            0
        }
    }

    fn cluster_size(
            &self
        ) -> usize
    {
        if self.sectors_per_cluster_shift < 32 {
            self.block_size() << self.sectors_per_cluster_shift
        } else {
            0
        }
    }

    fn block_to_offset(
            &self,
            block: u64
        ) -> u64
    {
        return block << self.bytes_per_sector_shift;
    }

    fn cluster_to_block(
            &self,
            cluster: u32
        ) -> u64
    {
        return u64::from(self.clustor_heap_offset) +
        (((cluster - EXFAT_FIRST_DATA_CLUSTER) as u64) << self.sectors_per_cluster_shift)
    }

    fn cluster_to_offset(
            &self,
            cluster: u32,
        ) -> u64
    {
        return self.block_to_offset(self.cluster_to_block(cluster));
    }

    fn next_cluster<R: Read+Seek>(
            &self,
            file: &mut R,
            cluster: u32,
        ) -> Result<u32, ExFatError>
    {
        let fat_offset = self.block_to_offset(u64::from(self.fat_offset))
                + (cluster as u64 * 4);
        let next: [u8; 4] = read_exact_at(file, fat_offset)?;
        
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

pub fn get_exfatcsum(
        sectors: &[u8],
        sector_size: usize,
    ) -> u32
{
    let n_bytes = sector_size * 11;

    let mut checksum: u32 = 0;

    for (i, byte) in sectors.iter().enumerate().take(n_bytes) {
        if i == 106 || i == 107 || i == 112 {
            continue;
        }

        checksum = checksum.rotate_right(1)
            .wrapping_add(*byte as u32);
    }

    return checksum;
}

fn verify_exfat_checksum(
        probe: &mut Probe,
        sb: ExFatSuperBlock
    ) -> Result<(), ExFatError>
{
    let sector_size = sb.block_size();
    let data = read_vec_at(&mut probe.file(), probe.offset(), sector_size * 12)?;
    let checksum = get_exfatcsum(&data, sector_size);
    
    for i in 0..(sector_size / 4) {
        let offset = sector_size * 11 + i * 4;
        if let Some(bytes) = data.get(offset..offset + 4) {
            let expected = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]); // FIX later

            if checksum != expected {
                return Err(ExFatError::ChecksumError { expected: CsumAlgorium::Exfat(expected), got: CsumAlgorium::Exfat(checksum) });
            }
        } else {
            return Err(ExFatError::ExfatHeaderError("Checksum buffer not big enough to read checksum")); 
        }
    }

    return Ok(());
}

#[inline]
fn in_range_inclusive<T: PartialOrd>(val: T, start: T, stop: T) -> bool {
    val >= start && val <= stop
}

fn valid_exfat(
        probe: &mut Probe,
        sb: ExFatSuperBlock
    ) -> Result<(), ExFatError>
{
    if u16::from(sb.boot_signature) != 0xAA55 {
        return Err(ExFatError::UnknownFilesystem("Block is not exfat likely a mbr partiton table"));
    }

    if sb.cluster_size() == 0 {
        return Err(ExFatError::ExfatHeaderError("Clustor size should not be 0"));
    }

    if sb.bootjmp != [0xEB, 0x76, 0x90] {
        return Err(ExFatError::ExfatHeaderError("No idea why boot jump should be \\xEB\\x76\\x90"));
    }

    if &sb.fs_name != b"EXFAT   " {
        return Err(ExFatError::ExfatHeaderError("fs_name should be \"EXFAT   \""));
    }

    if sb.must_be_zero != [0u8; 53] {
        return Err(ExFatError::ExfatHeaderError("must_be_zero region is not all zero"));
    }

    if !in_range_inclusive(sb.number_of_fats, 1, 2) {
        return Err(ExFatError::ExfatHeaderError("number of fats needs to be val >= 1 && val <= 2"));
    }

    if !in_range_inclusive(sb.bytes_per_sector_shift, 9, 12) {
        return Err(ExFatError::ExfatHeaderError("bytes_per_sector_shift needs to be val >= 9 && val <= 12"));
    }
    
    if !in_range_inclusive(sb.sectors_per_cluster_shift, 0, 25 - sb.bytes_per_sector_shift) {
        return Err(ExFatError::ExfatHeaderError("sectors_per_cluster_shift needs to be val >= 0 && val <= 25 - bytes_per_sector_shift"));
    }

    if !in_range_inclusive(
            u32::from(sb.fat_offset), 
            24, 
            u32::from(sb.clustor_heap_offset) - 
                    (u32::from(sb.fat_length) * sb.number_of_fats as u32)) 
    {
        return Err(ExFatError::ExfatHeaderError("fat_offset needs to be val >= 24 && val <= clustor_heap_offset - fat_length * number_of_fats "));
    }

    if !in_range_inclusive(
            u32::from(sb.clustor_heap_offset), 
            u32::from(sb.fat_offset) + 
                    u32::from(sb.fat_length) * sb.number_of_fats as u32,
            1u32 << (32 - 1)) 
    {
        return Err(ExFatError::ExfatHeaderError("clustor_heap_offset needs to be val >= fat_offset + fat_length * number_of_fats && val <= 1u32 << (32 - 1)"));
    }

    if !in_range_inclusive(
            u32::from(sb.first_clustor_of_root), 
            2, 
            u32::from(sb.clustor_count) + 1) 
    {
        return Err(ExFatError::ExfatHeaderError("first_clustor_of_root needs to be val >= 2 && val <= clustor_count + 1"));
    }

    verify_exfat_checksum(probe, sb)?;

    return Ok(());
}

pub fn probe_is_exfat(
        probe: &mut Probe
    ) -> Result<(), ExFatError>
{
    let sb: ExFatSuperBlock = from_file(&mut probe.file(), probe.offset())?;
    
    if probe_get_magic(&mut probe.file(), &VFAT_ID_INFO).is_ok() {
        return Err(ExFatError::UnknownFilesystem("Block is detected with a VFAT magic"));
    }

    valid_exfat(probe, sb)?;

    return Ok(());
}

fn find_label<R: Read+Seek>(
        file: &mut R, 
        sb: ExFatSuperBlock
    ) -> Result<Option<String>, ExFatError>
{
    let mut cluster = u32::from(sb.first_clustor_of_root);
    let mut offset = sb.cluster_to_offset(cluster);

    let mut i = 0;

    while i < 8388608 { // EXFAT_MAX_DIR_SIZE / EXFAT_ENTRY_SIZE
        let buf = match read_exact_at::<EXFAT_ENTRY_SIZE, R>(file, offset) {
            Ok(t) => t,
            Err(_) => {
                return Ok(None)
            }
        };

        let entry: ExfatEntryLabel = transmute!(buf);

        if entry.label_type == EXFAT_ENTRY_EOD {
            return Ok(None);
        }
        if entry.label_type == EXFAT_ENTRY_LABEL {
            let label = decode_utf16_lossy_from(&entry.name, Endianness::Little);
            return Ok(Some(label.to_string()));
        }

        offset += EXFAT_ENTRY_SIZE as u64;


        if sb.cluster_size() != 0 && offset.is_multiple_of(sb.cluster_size() as u64) {
            cluster = sb.next_cluster(file, cluster)?;
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

pub fn probe_exfat(
        probe: &mut Probe,
        _mag: BlockidMagic,
    ) -> Result<(), ExFatError> 
{
    let sb: ExFatSuperBlock = from_file(&mut probe.file(), probe.offset())?;

    valid_exfat(probe, sb)?;

    let label= find_label(&mut probe.file(), sb)?; 

    probe.push_result(
        ProbeResult::Filesystem(
            FilesystemResult {
                btype: Some(BlockType::Exfat),
                sec_type: None,
                uuid: Some(BlockidUUID::VolumeId32(VolumeId32::new(sb.volume_serial))),
                log_uuid: None,
                ext_journal: None,
                label,
                creator: None,
                usage: Some(UsageType::Filesystem),
                size: Some(sb.block_size() as u64 * u64::from(sb.volume_length)),
                fs_last_block: None, 
                fs_block_size: Some(sb.block_size() as u64), 
                block_size: Some(sb.block_size() as u64),
                version: Some(BlockidVersion::DevT(makedev(sb.vermaj as u32, sb.vermin as u32))),
                sbmagic: Some(b"EXFAT   "),
                sbmagic_offset: Some(3),
                endianness: None
            }
        ) 
    );
    return Ok(());
}