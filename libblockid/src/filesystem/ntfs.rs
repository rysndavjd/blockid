use fat_volume_id::VolumeId64;
use widestring::error::Utf16Error;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned,
    byteorder::{LittleEndian, U16, U32, U64},
    transmute_ref,
};

use crate::{
    error::Error,
    filesystem::{BlockInfo, BlockTag, BlockType},
    io::{BlockIo, Reader},
    probe::{Endianness, Id, Magic, ProbeFlags, Usage},
    std::fmt,
    util::{decode_utf16_from, decode_utf16_lossy_from},
};

#[derive(Debug, Clone)]
pub enum NtfsError {
    Utf16Error(Utf16Error),
    InvalidSectorSize,
    InvalidSectorPerCluster,
    ClusterSizeGreaterThanMax,
    UnusedFieldsNotZero,
    InvalidClustersPerMftRecord,
    InvalidMftRecordSizeShift,
    MftClusterLocationGreaterThanNrClusters,
    InvalidMftRecordSize,
    InvalidBufMftOneSignature,
    InvalidBufMftTwoSignature,
    InvalidLabelOffset,
    UnableToMapMasterFileTableRecord,
    UnableToMapFileAttribute,
}

impl fmt::Display for NtfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NtfsError::Utf16Error(e) => write!(f, "Filesystem label contains invalid UTF-16: {e}"),
            NtfsError::InvalidSectorSize => write!(f, "Invalid sector size"),
            NtfsError::InvalidSectorPerCluster => write!(f, "Invalid sector per cluster"),
            NtfsError::ClusterSizeGreaterThanMax => write!(f, "Cluster size greater than max"),
            NtfsError::UnusedFieldsNotZero => write!(f, "Unused fields not zero"),
            NtfsError::InvalidClustersPerMftRecord => write!(f, "Invalid clusters per mft record"),
            NtfsError::InvalidMftRecordSizeShift => write!(f, "Invalid mft record size shift"),
            NtfsError::MftClusterLocationGreaterThanNrClusters => {
                write!(f, "Mft cluster location greater than nr_clusters")
            }
            NtfsError::InvalidMftRecordSize => write!(f, "Invalid mft record size"),
            NtfsError::InvalidBufMftOneSignature => write!(f, "buf_mft 1 missing signature FILE"),
            NtfsError::InvalidBufMftTwoSignature => write!(f, "buf_mft 2 missing signature FILE"),
            NtfsError::InvalidLabelOffset => write!(f, "Invalid label offset"),
            NtfsError::UnableToMapMasterFileTableRecord => {
                write!(f, "Unable to `MapMasterFileTableRecord`")
            }
            NtfsError::UnableToMapFileAttribute => write!(f, "Unable to map `FileAttribute`"),
        }
    }
}

impl<E: fmt::Debug> From<NtfsError> for Error<E> {
    fn from(e: NtfsError) -> Self {
        Self::Ntfs(e)
    }
}

pub const NTFS_MINSZ: Option<u64> = None;
pub const NTFS_MAGICS: Option<&'static [Magic]> = Some(&[Magic {
    magic: b"NTFS    ",
    len: 8,
    b_offset: 3,
}]);

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

    pub unused: [U16<LittleEndian>; 2],
    pub number_of_sectors: U64<LittleEndian>,
    pub mft_cluster_location: U64<LittleEndian>,
    pub mft_mirror_cluster_location: U64<LittleEndian>,
    pub clusters_per_mft_record: i8,
    pub reserved1: [u8; 3],
    pub cluster_per_index_record: i8,
    pub reserved2: [u8; 3],
    pub volume_serial: [u8; 8],
    pub checksum: U32<LittleEndian>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout)]
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

const MFT_RECORD_ATTR_VOLUME_NAME: u32 = 0x60;
const MFT_RECORD_ATTR_END: u32 = 0xffffffff;

impl NtfsSuperBlock {
    fn check_ntfs(&self) -> Result<(u64, u64), NtfsError> // sector_size, sectors_per_cluster
    {
        let sector_size = u64::from(self.sector_size);

        if !(256..=4096).contains(&sector_size) || !sector_size.is_power_of_two() {
            return Err(NtfsError::InvalidSectorSize);
        }

        let sectors_per_cluster = match self.sectors_per_cluster {
            1 | 2 | 4 | 8 | 16 | 32 | 64 | 128 => u64::from(self.sectors_per_cluster),
            240..=249 => 1 << (256 - self.sectors_per_cluster as u16) as u8,
            _ => return Err(NtfsError::InvalidSectorPerCluster),
        };

        if (sector_size * sectors_per_cluster) > NTFS_MAX_CLUSTER_SIZE {
            return Err(NtfsError::ClusterSizeGreaterThanMax);
        }

        if u16::from(self.reserved_sectors) != 0
            || u16::from(self.root_entries) != 0
            || u16::from(self.sectors) != 0
            || u16::from(self.sectors_per_fat) != 0
            || u32::from(self.large_sectors) != 0
            || self.fats != 0
        {
            return Err(NtfsError::UnusedFieldsNotZero);
        }

        if (self.clusters_per_mft_record as u8) < 0xe1
            || (self.clusters_per_mft_record as u8) > 0xf7
                && matches!(self.clusters_per_mft_record, 1 | 2 | 4 | 8 | 16 | 32 | 64)
        {
            return Err(NtfsError::InvalidClustersPerMftRecord);
        }

        return Ok((sector_size, sectors_per_cluster));
    }

    fn find_label<IO: BlockIo>(
        &self,
        reader: &mut Reader<IO>,
        flags: ProbeFlags,
        sector_size: u64,
        sectors_per_cluster: u64,
    ) -> Result<Option<String>, Error<IO::Error>> {
        let mft_record_size = if self.clusters_per_mft_record > 0 {
            self.clusters_per_mft_record as u64 * sectors_per_cluster * sector_size
        } else {
            let mft_record_size_shift = 0 - self.clusters_per_mft_record;
            if !(0..31).contains(&mft_record_size_shift) {
                return Err(NtfsError::InvalidMftRecordSizeShift.into());
            }
            1 << mft_record_size_shift
        };

        let nr_clusters = u64::from(self.number_of_sectors) / sectors_per_cluster;

        if u64::from(self.mft_cluster_location) > nr_clusters
            || u64::from(self.mft_mirror_cluster_location) > nr_clusters
        {
            return Err(NtfsError::MftClusterLocationGreaterThanNrClusters.into());
        }

        let mut off = u64::from(self.mft_cluster_location) * sector_size * sectors_per_cluster;

        if mft_record_size < 4 {
            return Err(NtfsError::InvalidMftRecordSize.into());
        }

        let mut buf_mft = reader.read_vec_at(off, mft_record_size as usize)?;

        if &buf_mft[0..4] != b"FILE" {
            return Err(NtfsError::InvalidBufMftOneSignature.into());
        }

        off += MFT_RECORD_VOLUME * mft_record_size;

        buf_mft = reader.read_vec_at(off, mft_record_size as usize)?;

        if &buf_mft[0..4] != b"FILE" {
            return Err(NtfsError::InvalidBufMftTwoSignature.into());
        }

        let mft =
            MasterFileTableRecord::ref_from_bytes(&buf_mft[..size_of::<MasterFileTableRecord>()])
                .map_err(|_| NtfsError::UnableToMapMasterFileTableRecord)?;

        let mut attr_off = usize::from(mft.attrs_offset);

        while (attr_off + size_of::<FileAttribute>()) as u64 <= mft_record_size
            && attr_off as u64 <= u64::from(mft.bytes_allocated)
        {
            let attr = FileAttribute::ref_from_bytes(
                &buf_mft[attr_off..attr_off + size_of::<FileAttribute>()],
            )
            .map_err(|_| NtfsError::UnableToMapFileAttribute)?;

            let attr_len = u32::from(attr.len) as usize;

            if attr_len == 0 {
                break;
            }

            if u32::from(attr.file_type) == MFT_RECORD_ATTR_END {
                break;
            }

            if u32::from(attr.file_type) == MFT_RECORD_ATTR_VOLUME_NAME {
                let attr_bytes = &buf_mft[attr_off..attr_off + attr_len];

                let val_off = usize::from(attr.value_offset);
                let val_len = u64::from(attr.value_len);

                if attr_off as u64 + val_off as u64 + val_len <= mft_record_size {
                    let val = &attr_bytes[val_off..val_off + val_len as usize];

                    if val.is_empty() {
                        return Ok(None);
                    }

                    let label = if flags.contains(ProbeFlags::FailOnInvaildUTF) {
                        decode_utf16_from(val, Endianness::Little)
                            .map_err(NtfsError::Utf16Error)?
                            .to_string()
                    } else {
                        decode_utf16_lossy_from(val, Endianness::Little).to_string()
                    };

                    return Ok(Some(label));
                }
            }
            attr_off += attr_len;
        }

        return Err(NtfsError::InvalidLabelOffset.into());
    }
}

pub fn probe_is_ntfs<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
) -> Result<bool, Error<IO::Error>> {
    let buf: [u8; size_of::<NtfsSuperBlock>()] = reader.read_exact_at(offset)?;
    let sb: &NtfsSuperBlock = transmute_ref!(&buf);

    if reader
        .get_magic(NTFS_MAGICS.expect("NTFS magics is not `None`"))?
        .is_none()
    {
        return Ok(false);
    }

    match sb.check_ntfs() {
        Ok(_) => return Ok(true),
        Err(_) => return Ok(false),
    }
}

pub fn probe_ntfs<IO: BlockIo>(
    reader: &mut Reader<IO>,
    flags: ProbeFlags,
    offset: u64,
    mag: Magic,
) -> Result<BlockInfo, Error<IO::Error>> {
    let buf: [u8; size_of::<NtfsSuperBlock>()] = reader.read_exact_at(offset)?;

    let sb: &NtfsSuperBlock = transmute_ref!(&buf);

    let (sector_size, sectors_per_cluster) = sb.check_ntfs()?;

    let label = sb.find_label(reader, flags, sector_size, sectors_per_cluster)?;

    let mut info = BlockInfo::new();

    info.set(BlockTag::BlockType(BlockType::Ntfs));
    if let Some(label) = label {
        info.set(BlockTag::Label(label));
    }
    info.set(BlockTag::Id(Id::VolumeId64(VolumeId64::from_bytes(
        sb.volume_serial,
    ))));
    info.set(BlockTag::Usage(Usage::Filesystem));
    info.set(BlockTag::Magic(mag.magic.to_vec()));
    info.set(BlockTag::FsSize(u64::from(
        sb.number_of_sectors * sector_size,
    )));
    info.set(BlockTag::FsBlockSize(sector_size * sectors_per_cluster));
    info.set(BlockTag::BlockSize(sector_size));

    return Ok(info);
}
