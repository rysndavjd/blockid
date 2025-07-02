use std::io::{self, Read, Seek, SeekFrom, ErrorKind};

use bitflags::bitflags;
use zerocopy::{byteorder::{LittleEndian, U16, U32, U64}, transmute, FromBytes, Immutable, IntoBytes, Unaligned, Ref, KnownLayout};
use rustix::fs::makedev;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    checksum::{get_crc32c, 
    verify_crc32c, CsumAlgorium}, filesystems::{is_power_2, FsError}, from_buffer, from_file, probe_get_magic, read_exact_at, BlockidError, BlockidIdinfo, BlockidMagic, BlockidProbe, BlockidUUID, BlockidVersion, Endianness, FilesystemResults, FsType, ProbeResult, UsageType
};

#[derive(Error, Debug)]
pub enum NtfsError {
    #[error("I/O operation failed: {0}")]
    IoError(#[from] io::Error),
    #[error("Not an NTFS superblock: {0}")]
    UnknownFilesystem(&'static str),
    #[error("NTFS Header Error: {0}")]
    NtfsHeaderError(&'static str),
    #[error("NTFS Checksum failed, expected: \"{expected:X}\" and got: \"{got:X})\"")]
    ChecksumError {
        expected: CsumAlgorium,
        got: CsumAlgorium,
    }
}

impl From<NtfsError> for FsError {
    fn from(err: NtfsError) -> Self {
        match err {
            NtfsError::IoError(e) => FsError::IoError(e),
            NtfsError::NtfsHeaderError(info) => FsError::InvalidHeader(info),
            NtfsError::UnknownFilesystem(fs) => FsError::UnknownFilesystem(fs),
            NtfsError::ChecksumError { expected, got } => FsError::ChecksumError { expected, got },
        }
    }
}

pub const NTFS_ID_INFO: BlockidIdinfo = BlockidIdinfo {
    name: Some("ntfs"),
    usage: Some(UsageType::Filesystem),
    probe_fn: |probe, magic| {
        probe_ntfs(probe, magic)
        .map_err(FsError::from)
        .map_err(BlockidError::from)
    },
    minsz: None,
    magics: &[
        BlockidMagic {
            magic: b"NTFS    ",
            len: 8,
            b_offset: 3,
        },
    ]
};

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct NtfsSuperBlock {
    pub bootjmp: [u8; 3],
    pub oem_id: [u8; 8],

    pub sector_size: U16<LittleEndian>,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: U16<LittleEndian>,
    pub fats: u8,
    pub root_entries: U16<LittleEndian>,
    pub sectors: U16<LittleEndian>,
    pub media_type: u8,
    pub sectors_per_fat: U16<LittleEndian>,
    pub sectors_per_track: U16<LittleEndian>,
    pub heads: U16<LittleEndian>,
    pub hidden_sectors: U32<LittleEndian>,
    pub large_sectors: U32<LittleEndian>,

    pub unused: [u8; 2],
    pub number_of_sectors: U64<LittleEndian>,
    pub mft_cluster_location: U64<LittleEndian>,
    pub mft_mirror_cluster_location: U64<LittleEndian>,
    pub clusters_per_mft_record: i8,
    pub reserved1: [u8; 3],
    pub cluster_per_index_record: i8,
    pub reserved2: [u8; 3],
    pub volume_serial: U64<LittleEndian>,
    pub checksum: U32<LittleEndian>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
struct MasterFileTableRecord {
    pub magic: U32<LittleEndian>,
    pub usa_ofs: U16<LittleEndian>,
    pub usa_count: U16<LittleEndian>,
    pub lsn: U64<LittleEndian>,
    pub sequence_number: U16<LittleEndian>,
    pub link_count: U16<LittleEndian>,
    pub attrs_offset: U16<LittleEndian>,
    pub flags: U16<LittleEndian>,
    pub bytes_in_use: U32<LittleEndian>,
    pub bytes_allocated: U32<LittleEndian>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout)]
struct FileAttribute {
    pub file_type: U32<LittleEndian>,
    pub len: U32<LittleEndian>,
    pub non_resident: u8,
    pub name_len: u8,
    pub name_offset: U16<LittleEndian>,
    pub flags: U16<LittleEndian>,
    pub instance: U16<LittleEndian>,
    pub value_len: U32<LittleEndian>,
    pub value_offset: U16<LittleEndian>,
}

const MFT_RECORD_VOLUME: u64 = 3;
const NTFS_MAX_CLUSTER_SIZE: u64 = 2097152; //2 * 1024 * 1024

const MFT_RECORD_ATTR_VOLUME_NAME: u64 = 0x60;
const MFT_RECORD_ATTR_END: u64 = 0xffffffff;


fn check_ntfs(
        ns: NtfsSuperBlock,
    ) -> Result<(u64, u64), NtfsError> // sector_size, sectors_per_cluster
{    
    let sector_size = u64::from(ns.sector_size);

    if sector_size < 256 || sector_size > 4096 || !is_power_2(sector_size) {
        return Err(NtfsError::NtfsHeaderError("Sector size is wrong"));
    }

    let sectors_per_cluster = match ns.sectors_per_cluster {
        1 | 2 | 4 | 8 | 16 | 32 | 64 | 128 => u64::from(ns.sectors_per_cluster),
        240..=249 => 1 << (256 - ns.sectors_per_cluster as u16) as u8,
        _ => return Err(NtfsError::NtfsHeaderError("Sector Per Cluster wrong")),
    };

    if (sector_size * sectors_per_cluster) > NTFS_MAX_CLUSTER_SIZE {
        return Err(NtfsError::NtfsHeaderError("Too mant clusters"));
    }

    if u16::from(ns.reserved_sectors) != 0 
    || u16::from(ns.root_entries) != 0
    || u16::from(ns.sectors) != 0
    || u16::from(ns.sectors_per_fat) != 0
    || u32::from(ns.large_sectors) != 0
    || u8::from(ns.fats) != 0 {
        return Err(NtfsError::NtfsHeaderError("Unused fields must be zero"));
    }

    if (ns.clusters_per_mft_record as u8) < 0xe1
    || (ns.clusters_per_mft_record as u8) > 0xf7 {
        if matches!(ns.clusters_per_mft_record, 1 | 2 | 4 | 8 | 16 | 32 | 64) {
            return Err(NtfsError::NtfsHeaderError("wrong value: clusters_per_mft_record"))
        }
    }

    return Ok((sector_size, sectors_per_cluster));
}


fn get_id<R: Read+Seek>(
        file: &mut R,
        ns: NtfsSuperBlock,
        sector_size: u64,
        sectors_per_cluster: u64,
    ) -> Result<(), NtfsError>
{
    let mft_record_size = if ns.clusters_per_mft_record > 0 {
        ns.clusters_per_mft_record as u64 * sectors_per_cluster * sector_size
    } else {
        let mft_record_size_shift = 0 - ns.clusters_per_mft_record;
        if mft_record_size_shift < 0 || mft_record_size_shift >= 31 {
            return Err(NtfsError::NtfsHeaderError("mft_record_size_shift is out of range"));
        }
        1 << mft_record_size_shift
    };

    println!("mft_record_size: getid: {}", mft_record_size);

    let nr_clusters = u64::from(ns.number_of_sectors) / sectors_per_cluster;

    println!("nr_clusters: {}", nr_clusters);

    if u64::from(ns.mft_cluster_location) > nr_clusters ||
        u64::from(ns.mft_mirror_cluster_location) > nr_clusters {
        return Err(NtfsError::NtfsHeaderError("Eh some error, look at the source code 1"));
    }

    let mut off = u64::from(ns.mft_cluster_location) * sector_size * sectors_per_cluster;

    if mft_record_size < 4 {
        return Err(NtfsError::NtfsHeaderError("Eh some error, look at the source code 2"));
    }

    let mut buf_mft: [u8; 32] = read_exact_at(file, off)?;

    if &buf_mft[0..3] == b"FILE" {
        return Err(NtfsError::NtfsHeaderError("Eh some error, look at the source code 3"));
    }

    off += MFT_RECORD_VOLUME * mft_record_size;

    buf_mft = read_exact_at(file, off)?;

    if &buf_mft[0..3] == b"FILE" {
        return Err(NtfsError::NtfsHeaderError("Eh some error, look at the source code 4"));
    }

    let mft: MasterFileTableRecord = transmute!(buf_mft);
    let mut attr_off = usize::from(mft.attrs_offset);

    while (attr_off + 22) as u64 <= mft_record_size && 
    attr_off as u64 <= u64::from(mft.bytes_allocated) {
        
        let attr: FileAttribute = from_buffer(&buf_mft, attr_off)?;
        let attr_len = u32::from(attr.len);

        if attr_len == 0 {
            break;
        }

        if u64::from(attr.file_type) == MFT_RECORD_ATTR_END {
            break;
        }

        if u64::from(attr.file_type) == MFT_RECORD_ATTR_VOLUME_NAME {
            let val_off = usize::from(attr.value_offset);
            let val_len = u32::from(attr.value_len);
            let val = &attr.as_bytes()[val_off..];
            
            if attr_off as u64 + val_off as u64 + u64::from(val_len) <= mft_record_size {
                println!("{:X?}", val)
            }
            break;
        }

        attr_off += attr_len as usize;
    }

    return Ok(());
}


pub fn probe_is_ntfs<R: Read+Seek>(
        file: &mut R,
    ) -> Result<(), NtfsError>
{
    let ns: NtfsSuperBlock = from_file(file, 0)?;
    
    let _ = probe_get_magic(file, &NTFS_ID_INFO)?;
    
    let _ = check_ntfs(ns)?;

    return Ok(());
}

pub fn probe_ntfs(
        probe: &mut BlockidProbe, 
        magic: BlockidMagic
    ) -> Result<(), NtfsError> 
{
    let ns: NtfsSuperBlock = from_file(&mut probe.file, 0)?;

    println!("{:X?}", ns);

    let (sector_size, sectors_per_cluster) = check_ntfs(ns)?;

    println!("sector_size: {}", sector_size);
    println!("sectors_per_cluster: {}", sectors_per_cluster);

    get_id(&mut probe.file, ns, sector_size, sectors_per_cluster)?;

/* 
    probe.push_result(ProbeResult::Filesystem(
            FilesystemResults { 
                fs_type: Some(FsType::Ntfs), 
                sec_type: None, 
                label: label, 
                fs_uuid: Some(BlockidUUID::VolumeId32(serno)), 
                log_uuid: None, 
                ext_journal: None, 
                fs_creator: None, 
                usage: Some(UsageType::Filesystem), 
                version: None, 
                sbmagic: Some(magic.magic), 
                sbmagic_offset: Some(magic.b_offset), 
                fs_size: Some(u64::from(ns.number_of_sectors) * sector_size), 
                fs_last_block: None, 
                fs_block_size: Some(sector_size * sectors_per_cluster), 
                block_size: Some(sector_size),
                endianness: None,
            }
        )
    );
*/
    return Ok(());
}