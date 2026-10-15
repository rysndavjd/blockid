use bitflags::bitflags;
use crc::{CRC_32_ISO_HDLC, Crc};
use uuid::Uuid;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, LittleEndian, U16, U32, U64, Unaligned,
    transmute_ref,
};

use crate::{
    Endianness,
    error::Error,
    io::Reader,
    partition::{
        BlockIo, Partition, PartitionAttributes, PartitionId, PartitionType, PtInfo, PtType,
    },
    probe::Magic,
    std::{fmt, mem::offset_of},
    util::bytes_to_u16string,
};

/// Errors that may occur when parsing GPT (GUID Partition Table).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum GptError {
    /// Sector size cannot be detected with `get_lssz_manual` and `os_calls` is
    /// unavailable.
    UnableToGetSectorSize,
    /// Header signature is invalid.
    InvalidSignature,
    /// Header size is below 92 bytes or exceeds the disk sector size.
    InvalidHeaderSize,
    /// Stored header checksum does not match checksum calculated from
    /// header contents.
    InvalidHeaderChecksum,
    /// Header LBA location does not match its actual location on disk.
    MismatchMyLBA,
    /// Header usable LBA range is invalid.
    InvalidLbaUsableRegions,
    /// Partition entries are invalid.
    GptEntriesUndefined,
    /// Stored entries checksum does not match checksum calculated from
    /// entry contents.
    InvalidGptEntriesChecksum,
    /// Calculating the header position overflowed.
    HeaderPositionCalculation,
    /// Calculating the last LBA overflowed
    LastLbaCalculation,
}

impl fmt::Display for GptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GptError::UnableToGetSectorSize => write!(
                f,
                "sector size cannot be detected with `get_lssz_manual` and `os_calls` is unavailable"
            ),
            GptError::InvalidSignature => write!(f, "invalid GPT header signature"),
            GptError::InvalidHeaderSize => write!(
                f,
                "header size is below 92 bytes or exceeds the disk sector size"
            ),
            GptError::InvalidHeaderChecksum => write!(
                f,
                "stored header checksum does not match checksum calculated from header contents"
            ),
            GptError::MismatchMyLBA => write!(
                f,
                "header LBA location does not match its actual location on disk"
            ),
            GptError::InvalidLbaUsableRegions => {
                write!(f, "header usable LBA range is invalid")
            }
            GptError::GptEntriesUndefined => write!(f, "partition entries are invalid"),
            GptError::InvalidGptEntriesChecksum => write!(
                f,
                "stored entries checksum does not match checksum calculated from entry contents"
            ),
            GptError::HeaderPositionCalculation => {
                write!(f, "calculating the header position overflowed")
            }
            GptError::LastLbaCalculation => write!(f, "calculating the last LBA overflowed"),
        }
    }
}

impl<E: fmt::Debug> From<GptError> for Error<E> {
    fn from(e: GptError) -> Self {
        Error::Gpt(e)
    }
}

pub(crate) const GPT_MINSZ: Option<u64> = Some(32768);
pub(crate) const GPT_MAGICS: Option<&'static [Magic]> = None;

/// A GUID as in the disk layout used by GPT and UEFI, as opposed to the
/// big endian layout of [`uuid::Uuid`].
#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, PartialEq)]
pub(crate) struct EfiGuid {
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
            uuid.time_low.get(),
            uuid.time_mid.get(),
            uuid.time_hi_and_version.get(),
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

/// Attribute flags of a GPT partition entry
///
/// * Bit 0: [`REQUIRED_PARTITION`][Self::REQUIRED_PARTITION]
/// * Bit 1: [`NO_BLOCK_IO_PROTOCOL`][Self::NO_BLOCK_IO_PROTOCOL]
/// * Bit 2: [`LEGACY_BIOS_BOOTABLE`][Self::LEGACY_BIOS_BOOTABLE]
/// * Bits `3..=47`: reserved for future use.
/// * Bits `48..=63`: defined by the individual partition type.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct GptAttributes(u64);

bitflags! {
    impl GptAttributes: u64 {
        /// The partition is required for the platform to function.
        const REQUIRED_PARTITION   = 1 << 0;
        /// Firmware must not produce `EFI_BLOCK_IO_PROTOCOL` for this partition.
        const NO_BLOCK_IO_PROTOCOL = 1 << 1;
        /// The partition can be booted by legacy BIOS firmware.
        const LEGACY_BIOS_BOOTABLE = 1 << 2;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout)]
pub(crate) struct GptTable {
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
pub(crate) struct GptEntry {
    partition_type_guid: EfiGuid,
    unique_partition_guid: EfiGuid,
    starting_lba: U64<LittleEndian>,
    ending_lba: U64<LittleEndian>,

    attributes: U64<LittleEndian>,
    partition_name: [u8; 72],
}

impl GptEntry {
    #[inline]
    fn attributes(&self) -> GptAttributes {
        GptAttributes::from_bits_retain(self.attributes.get())
    }
}

impl GptTable {
    const SIGNATURE: u64 = 0x5452415020494645;
    const SIGNATURE_STR: &[u8] = b"EFI PART";
    const MIN_HEADER_SIZE: u64 = 92;
    const FIRST_LBA: u64 = 1;
    const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

    fn get_lssz_manual<IO: BlockIo>(
        reader: &mut Reader<IO>,
        offset: u64,
    ) -> Result<u64, Error<IO::Error>> {
        for lssz in [512, 1024, 2048, 4096, 8192] {
            let pos = match offset.checked_add(lssz) {
                Some(t) => t,
                None => continue,
            };

            if let Ok(sig) = reader.read_exact_at::<8>(pos)
                && (sig == Self::SIGNATURE_STR)
            {
                return Ok(lssz);
            }
        }
        Err(GptError::UnableToGetSectorSize.into())
    }

    fn get_header<IO: BlockIo>(
        reader: &mut Reader<IO>,
        offset: u64,
        lba: u64,
        last_lba: u64,
        lssz: u64,
        device_size: u64,
    ) -> Result<(GptTable, Vec<u8>), Error<IO::Error>> {
        let header_pos = lba
            .checked_mul(lssz)
            .and_then(|v| v.checked_add(offset))
            .ok_or(GptError::HeaderPositionCalculation)?;

        let buf = reader.read_vec_at(header_pos, lssz as usize)?;

        let header_buf = buf
            .first_chunk::<{ size_of::<GptTable>() }>()
            .ok_or(GptError::InvalidHeaderSize)?;
        let header: &GptTable = transmute_ref!(header_buf);

        if u64::from(header.signature) != GptTable::SIGNATURE {
            return Err(GptError::InvalidSignature.into());
        }

        let hsz = u64::from(header.header_size);
        if hsz < GptTable::MIN_HEADER_SIZE || hsz > lssz {
            return Err(GptError::InvalidHeaderSize.into());
        }

        let header_crc = header.header_crc32.get();
        let mut hdr = buf[..hsz as usize].to_vec();
        hdr[offset_of!(GptTable, header_crc32)..][..4].fill(0);

        if header_crc != Self::CRC.checksum(&hdr) {
            return Err(GptError::InvalidHeaderChecksum.into());
        }

        if u64::from(header.my_lba) != lba {
            return Err(GptError::MismatchMyLBA.into());
        }

        let fu = u64::from(header.first_usable_lba);
        let lu = u64::from(header.last_usable_lba);

        if lu < fu || fu > last_lba || lu > last_lba {
            return Err(GptError::InvalidLbaUsableRegions.into());
        }

        let entry_sz = header.sizeof_partition_entry.get();
        let num_entries = header.num_partition_entries.get();

        if num_entries == u32::MIN || entry_sz != (size_of::<GptEntry>() as u32) {
            return Err(GptError::GptEntriesUndefined.into());
        }

        let entries_sz = num_entries
            .checked_mul(entry_sz)
            .ok_or(GptError::GptEntriesUndefined)?;
        let entries_start = u64::from(header.partition_entries_lba)
            .checked_mul(lssz)
            .ok_or(GptError::GptEntriesUndefined)?;
        let entries_end = entries_start
            .checked_add(entries_sz as u64)
            .ok_or(GptError::GptEntriesUndefined)?;

        if entries_end > device_size {
            return Err(GptError::GptEntriesUndefined.into());
        }

        let entries_buf = reader.read_vec_at(entries_start, entries_sz as usize)?;

        if header.partition_entry_array_crc32.get() != Self::CRC.checksum(&entries_buf) {
            return Err(GptError::InvalidGptEntriesChecksum.into());
        }

        return Ok((*header, entries_buf));
    }
}

/// When `os_calls` is unavailable the logical sector size of the disk is
/// found manually by scanning for the GPT signature string which is located
/// at offset of the devices logical sector size.
///
/// When `os_calls` is available logical sectors size of the disk is found
/// via `os_calls`.
pub(crate) fn probe_gpt<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    _: Magic,
) -> Result<PtInfo, Error<IO::Error>> {
    #[cfg(feature = "os_calls")]
    let (lssz, device_size) = if reader.os_calls() {
        (reader.logical_sector_size()?, reader.device_size()?)
    } else {
        (
            GptTable::get_lssz_manual(reader, offset)?,
            reader.seek(crate::io::SeekFrom::End(0))?,
        )
    };

    #[cfg(not(feature = "os_calls"))]
    let (lssz, device_size) = (
        GptTable::get_lssz_manual(reader, offset)?,
        reader.seek(crate::io::SeekFrom::End(0))?,
    );

    let device_size = device_size
        .checked_sub(offset)
        .ok_or(Error::OffsetExceedsDeviceSize)?;

    let last_lba = device_size
        .checked_div(lssz)
        .and_then(|n| n.checked_sub(1))
        .ok_or(GptError::LastLbaCalculation)?;

    let (header, entries_buf) = match GptTable::get_header(
        reader,
        offset,
        GptTable::FIRST_LBA,
        last_lba,
        lssz,
        device_size,
    ) {
        Ok((header, entries_buf)) => (header, entries_buf),
        Err(_) => GptTable::get_header(reader, offset, last_lba, last_lba, lssz, device_size)?,
    };

    let fu = header.first_usable_lba.get();
    let lu = header.last_usable_lba.get();

    let mut partitions: Vec<Partition> = Vec::new();
    let mut part_no = 1;

    for entry_buf in entries_buf.as_chunks::<{ size_of::<GptEntry>() }>().0 {
        let partition: &GptEntry = transmute_ref!(entry_buf);

        if partition.partition_type_guid == EfiGuid::ZERO {
            continue;
        }

        let start = partition.starting_lba.get();
        let end = partition.ending_lba.get();

        if start < fu || end > lu {
            continue;
        }

        let name = if partition.partition_name != [0u8; 72] {
            let label = bytes_to_u16string(&partition.partition_name, Endianness::Little);

            Some(label.into())
        } else {
            None
        };

        partitions.push(Partition {
            start: start * lssz,
            end: end * lssz,
            partition_id: PartitionId::Uuid(partition.unique_partition_guid.into()),
            partition_type: PartitionType::Uuid(partition.partition_type_guid.into()),
            part_no,
            partition_name: name,
            attributes: PartitionAttributes::Gpt(partition.attributes()),
        });
        part_no += 1;
    }

    let mut info = PtInfo::empty();

    info.set_pt_type(PtType::Gpt);
    info.set_pt_id(Uuid::from(header.disk_guid).into());
    info.set_pt_size((u64::from(header.alternate_lba) + 1) * lssz);
    info.set_magic(GptTable::SIGNATURE_STR.to_vec(), lssz);
    if !partitions.is_empty() {
        info.set_partitions(partitions);
    }

    return Ok(info);
}
