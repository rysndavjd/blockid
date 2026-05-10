use uuid::Uuid;
use widestring::error::Utf16Error;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, LittleEndian, TryFromBytes, U16, U32, U64,
    Unaligned,
};

use crate::{
    Endianness, Id, PTType, PartTableTag,
    error::Error,
    io::Reader,
    partition::{BlockIo, PartAttributes, PartId, PartTableInfo, PartType, Partition},
    probe::{Magic, ProbeFlags},
    std::mem::offset_of,
    util::{decode_utf16_from, decode_utf16_lossy_from},
};

#[derive(Debug, Clone)]
pub enum GptError {
    Utf16Error { error: Utf16Error, part_no: u64 },
    UnableToMapHeaderStruct,
    UnableToMapPartitionStruct { part_no: u64 },
    UnableToGetSectorSize,
    InvalidSignature,
    InvalidHeaderSize,
    InvalidHeaderChecksum,
    MismatchMyLBA,
    InvalidLbaUsableRegions,
    GptEntriesUndefined,
    InvalidGptEntriesChecksum,
}

impl core::fmt::Display for GptError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GptError::Utf16Error { error, part_no } => {
                write!(
                    f,
                    "Partition {part_no} label contains invalid UTF-16: {error}"
                )
            }
            GptError::UnableToMapHeaderStruct => {
                write!(f, "Unable to map bytes to `GptTable` struct")
            }
            GptError::UnableToMapPartitionStruct { part_no } => {
                write!(
                    f,
                    "Unable to map partition {part_no} bytes to `GptEntry` struct"
                )
            }
            GptError::UnableToGetSectorSize => {
                write!(f, "Unable to get sector size of partition table")
            }
            GptError::InvalidSignature => {
                write!(f, "Invalid signature found")
            }
            GptError::InvalidHeaderSize => {
                write!(f, "Invalid header size")
            }
            GptError::InvalidHeaderChecksum => {
                write!(f, "Invalid header checksum")
            }
            GptError::MismatchMyLBA => {
                write!(f, "Header has mismatch `my_lba` compared to real position")
            }
            GptError::InvalidLbaUsableRegions => {
                write!(f, "`{{first/last}}_usable_lba` out of range")
            }
            GptError::GptEntriesUndefined => {
                write!(f, "GPT entries have undefined size")
            }
            GptError::InvalidGptEntriesChecksum => {
                write!(f, "GPT entries have invalid checksum")
            }
        }
    }
}

impl<E: core::fmt::Debug> From<GptError> for Error<E> {
    fn from(e: GptError) -> Self {
        Error::Gpt(e)
    }
}

pub const GPT_MINSZ: Option<u64> = Some(32768);
pub const GPT_MAGICS: Option<&'static [Magic]> = None;

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, PartialEq)]
pub struct EfiGuid {
    time_low: U32<LittleEndian>,
    time_mid: U16<LittleEndian>,
    time_hi_and_version: U16<LittleEndian>,
    clock_seq_hi: u8,
    clock_seq_low: u8,
    node: [u8; 6],
}

impl EfiGuid {
    const ZERO: EfiGuid = EfiGuid {
        time_low: U32::new(0),
        time_mid: U16::new(0),
        time_hi_and_version: U16::new(0),
        clock_seq_hi: 0,
        clock_seq_low: 0,
        node: [0u8; 6],
    };
}

impl From<EfiGuid> for Uuid {
    fn from(uuid: EfiGuid) -> Self {
        Uuid::from_fields(
            u32::from(uuid.time_low),
            u16::from(uuid.time_mid),
            u16::from(uuid.time_hi_and_version),
            &[
                uuid.clock_seq_hi,
                uuid.clock_seq_low,
                uuid.node[0],
                uuid.node[1],
                uuid.node[2],
                uuid.node[3],
                uuid.node[4],
                uuid.node[5],
            ],
        )
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, TryFromBytes, IntoBytes, Unaligned, Immutable, KnownLayout)]
pub struct GptTable {
    pub signature: U64<LittleEndian>,
    pub revision: U32<LittleEndian>,
    pub header_size: U32<LittleEndian>,
    pub header_crc32: U32<LittleEndian>,

    pub reserved1: U32<LittleEndian>,

    pub my_lba: U64<LittleEndian>,
    pub alternate_lba: U64<LittleEndian>,
    pub first_usable_lba: U64<LittleEndian>,
    pub last_usable_lba: U64<LittleEndian>,

    pub disk_guid: EfiGuid,

    pub partition_entries_lba: U64<LittleEndian>,
    pub num_partition_entries: U32<LittleEndian>,
    pub sizeof_partition_entry: U32<LittleEndian>,
    pub partition_entry_array_crc32: U32<LittleEndian>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout)]
pub struct GptEntry {
    partition_type_guid: EfiGuid,
    unique_partition_guid: EfiGuid,
    starting_lba: U64<LittleEndian>,
    ending_lba: U64<LittleEndian>,

    attributes: U64<LittleEndian>,
    partition_name: [u8; 72],
}

impl GptTable {
    /// The offset used that is read off the disk to find the GPT header and its block size.
    const GPT_DETECT_OFFSET: usize = 16384;
    const SIGNATURE: u64 = 0x5452415020494645;
    const SIGNATURE_STR: &[u8] = b"EFI PART";
    const MIN_HEADER_SIZE: u64 = 92;
    const FIRST_LBA: u64 = 1;

    #[cfg(feature = "os_calls")]
    fn get_header<IO: BlockIo>(
        reader: &mut Reader<IO>,
        offset: u64,
        lba: u64,
        last_lba: u64,
        lssz: u64,
    ) -> Result<(GptTable, Vec<u8>), Error<IO::Error>> {
        let buf = reader.read_vec_at(offset + (lba * lssz), lssz as usize)?;

        let header: &GptTable = GptTable::try_ref_from_bytes(&buf[..size_of::<GptTable>()])
            .map_err(|_| GptError::UnableToMapHeaderStruct)?;

        if u64::from(header.signature) != GptTable::SIGNATURE {
            return Err(GptError::InvalidSignature.into());
        }

        let hsz = u64::from(header.header_size);

        if hsz < GptTable::MIN_HEADER_SIZE || hsz > lssz {
            return Err(GptError::InvalidHeaderSize.into());
        }

        let stored_crc = u32::from(header.header_crc32);

        let mut hdr = header.as_bytes().to_vec();
        hdr[offset_of!(GptTable, header_crc32)..offset_of!(GptTable, header_crc32) + 4].fill(0);

        #[cfg(feature = "std")]
        {
            let calc_crc = crc_fast::crc32_iso_hdlc(&hdr);

            if stored_crc != calc_crc {
                return Err(GptError::InvalidHeaderChecksum.into());
            };
        }

        #[cfg(feature = "no_std")]
        {
            use crc::{CRC_32_ISO_HDLC, Crc};

            let crc = Crc::<u32>::new(&CRC_32_ISO_HDLC);
            let mut digest = crc.digest();

            digest.update(&hdr);

            if stored_crc != digest.finalize() {
                return Err(GptError::InvalidGptEntriesChecksum.into());
            }
        }

        if u64::from(header.my_lba) != lba {
            return Err(GptError::MismatchMyLBA.into());
        }

        let fu = u64::from(header.first_usable_lba);
        let lu = u64::from(header.last_usable_lba);

        if lu < fu || fu > last_lba || lu > last_lba {
            return Err(GptError::InvalidLbaUsableRegions.into());
        }

        let entry_sz = u64::from(header.sizeof_partition_entry);
        let entries_sz = u64::from(header.num_partition_entries) * entry_sz;

        if entries_sz == 0
            || entries_sz >= u32::MAX as u64
            || entry_sz != size_of::<GptEntry>() as u64
        {
            return Err(GptError::GptEntriesUndefined.into());
        }

        let entries_buf = reader.read_vec_at(
            offset + (u64::from(header.partition_entries_lba) * lssz),
            entries_sz as usize,
        )?;

        #[cfg(feature = "std")]
        {
            let calc_crc = crc_fast::crc32_iso_hdlc(&entries_buf);

            if calc_crc != u32::from(header.partition_entry_array_crc32) {
                return Err(GptError::InvalidGptEntriesChecksum.into());
            }
        }

        #[cfg(feature = "no_std")]
        {
            use crc::{CRC_32_ISO_HDLC, Crc};

            let crc = Crc::<u32>::new(&CRC_32_ISO_HDLC);
            let mut digest = crc.digest();

            digest.update(&entries_buf);

            if u32::from(header.partition_entry_array_crc32) != digest.finalize() {
                return Err(GptError::InvalidGptEntriesChecksum.into());
            }
        }

        return Ok((*header, entries_buf));
    }
}

/// When `os_calls` is unavailable only the primary header can detected and
/// parsed for its infomation but in unlikely case that primary header is
/// corrupted this implementation will not be able to detect the secondary
/// headers locations and use its contents instead.
///
/// When `os_calls` is available then secondary header will parsed if an error
/// is detected with the primary header, as additional infomation can be used
/// from `os_calls` of device size to get the secondary header location on
/// disk.
pub fn probe_gpt<IO: BlockIo>(
    reader: &mut Reader<IO>,
    flags: ProbeFlags,
    offset: u64,
    _: Magic,
) -> Result<PartTableInfo, Error<IO::Error>> {
    #[cfg(not(feature = "os_calls"))]
    let (header, entries_buf, lssz) = {
        let buf: [u8; GptTable::GPT_DETECT_OFFSET] = reader.read_exact_at(offset)?;

        let lssz = buf
            .chunks_exact(GptTable::SIGNATURE_STR.len())
            .enumerate()
            .take_while(|(i, _)| i * GptTable::SIGNATURE_STR.len() < GptTable::GPT_DETECT_OFFSET)
            .find_map(|(i, raw)| {
                if raw == GptTable::SIGNATURE_STR {
                    Some(i * GptTable::SIGNATURE_STR.len())
                } else {
                    None
                }
            })
            .ok_or(GptError::UnableToGetSectorSize)?;

        let header: &GptTable =
            GptTable::try_ref_from_bytes(&buf[lssz..(lssz + size_of::<GptTable>())])
                .map_err(|_| GptError::UnableToMapHeaderStruct)?;

        if u64::from(header.signature) != GptTable::SIGNATURE {
            return Err(GptError::InvalidSignature.into());
        }

        let hsz = u64::from(header.header_size);

        if hsz < GptTable::MIN_HEADER_SIZE || hsz > lssz as u64 {
            return Err(GptError::InvalidHeaderSize.into());
        }

        let stored_crc = u32::from(header.header_crc32);

        let mut hdr = header.as_bytes().to_vec();
        hdr[offset_of!(GptTable, header_crc32)..offset_of!(GptTable, header_crc32) + 4].fill(0);

        #[cfg(feature = "std")]
        {
            let calc_crc = crc_fast::crc32_iso_hdlc(&hdr);

            if stored_crc != calc_crc {
                return Err(GptError::InvalidHeaderChecksum.into());
            };
        }

        #[cfg(feature = "no_std")]
        {
            use crc::{CRC_32_ISO_HDLC, Crc};

            let crc = Crc::<u32>::new(&CRC_32_ISO_HDLC);
            let mut digest = crc.digest();

            digest.update(&hdr);

            if stored_crc != digest.finalize() {
                return Err(GptError::InvalidGptEntriesChecksum.into());
            }
        }

        if u64::from(header.my_lba) != GptTable::FIRST_LBA {
            return Err(GptError::MismatchMyLBA.into());
        }

        let last_lba = u64::from(header.alternate_lba);

        let fu = u64::from(header.first_usable_lba);
        let lu = u64::from(header.last_usable_lba);

        if lu < fu || fu > last_lba || lu > last_lba {
            return Err(GptError::InvalidLbaUsableRegions.into());
        }

        let entry_sz = u64::from(header.sizeof_partition_entry);
        let entries_sz = u64::from(header.num_partition_entries) * entry_sz;

        if entries_sz == 0
            || entries_sz >= u32::MAX as u64
            || entry_sz != size_of::<GptEntry>() as u64
        {
            return Err(GptError::GptEntriesUndefined.into());
        }

        let entries_buf = reader.read_vec_at(
            offset + (u64::from(header.partition_entries_lba) * lssz as u64),
            entries_sz as usize,
        )?;

        #[cfg(feature = "std")]
        {
            let calc_crc = crc_fast::crc32_iso_hdlc(&entries_buf);

            if calc_crc != u32::from(header.partition_entry_array_crc32) {
                return Err(GptError::InvalidGptEntriesChecksum.into());
            }
        }

        #[cfg(feature = "no_std")]
        {
            use crc::{CRC_32_ISO_HDLC, Crc};

            let crc = Crc::<u32>::new(&CRC_32_ISO_HDLC);
            let mut digest = crc.digest();

            digest.update(&entries_buf);

            if u32::from(header.partition_entry_array_crc32) != digest.finalize() {
                return Err(GptError::InvalidGptEntriesChecksum.into());
            }
        }

        (*header, entries_buf, lssz as u64)
    };

    #[cfg(feature = "os_calls")]
    let (header, entries_buf, lssz) = {
        let lssz = reader.logical_sector_size()?;
        let last_lba = (reader.device_size()? / lssz) - 1;

        let (header, entries_buf) =
            match GptTable::get_header(reader, offset, GptTable::FIRST_LBA, last_lba, lssz) {
                Ok((header, entries_buf)) => (header, entries_buf),
                Err(_) => match GptTable::get_header(reader, offset, last_lba, last_lba, lssz) {
                    Ok((header, entries_buf)) => (header, entries_buf),
                    Err(e) => return Err(e),
                },
            };

        (header, entries_buf, lssz)
    };

    let fu = u64::from(header.first_usable_lba);
    let lu = u64::from(header.last_usable_lba);

    let mut partitions: Vec<Partition> = Vec::new();
    for i in 0..u64::from(header.num_partition_entries) {
        let partition = match GptEntry::ref_from_bytes(
            &entries_buf[i as usize * size_of::<GptEntry>()
                ..(i as usize * size_of::<GptEntry>()) + size_of::<GptEntry>()],
        ) {
            Ok(p) => p,
            Err(_) => return Err(GptError::UnableToMapPartitionStruct { part_no: i + 1 }.into()),
        };

        if partition.unique_partition_guid == EfiGuid::ZERO {
            continue;
        }

        let start = u64::from(partition.starting_lba);
        let end = u64::from(partition.ending_lba);

        if start < fu || end > lu {
            continue;
        }

        let name = if partition.partition_name != [0u8; 72] {
            if flags.contains(ProbeFlags::FailOnInvaildUTF) {
                match decode_utf16_from(&partition.partition_name, Endianness::Little) {
                    Ok(t) => Some(t.to_string()),
                    Err(e) => {
                        return Err(GptError::Utf16Error {
                            error: e,
                            part_no: i + 1,
                        }
                        .into());
                    }
                }
            } else {
                Some(
                    decode_utf16_lossy_from(&partition.partition_name, Endianness::Little)
                        .to_string(),
                )
            }
        } else {
            None
        };

        partitions.push(Partition {
            start: start * lssz,
            end: end * lssz,
            partition_id: PartId::Uuid(partition.unique_partition_guid.into()),
            partition_type: PartType::Uuid(partition.partition_type_guid.into()),
            part_no: i + 1,
            partition_name: name,
            attributes: PartAttributes::Gpt(u64::from(partition.attributes)),
        });
    }

    let mut info = PartTableInfo::new();

    info.set(PartTableTag::PtType(PTType::Gpt));
    info.set(PartTableTag::PtId(Id::Uuid(header.disk_guid.into())));
    info.set(PartTableTag::PtSize(
        (u64::from(header.alternate_lba) + 1) * lssz,
    ));
    info.set(PartTableTag::Magic(GptTable::SIGNATURE_STR.to_vec()));
    info.set(PartTableTag::MagicOffset(lssz));
    if !partitions.is_empty() {
        info.set(PartTableTag::Partions(partitions));
    }

    return Ok(info);
}
