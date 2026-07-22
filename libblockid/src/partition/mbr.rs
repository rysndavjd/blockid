use bitflags::bitflags;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned,
    byteorder::{LittleEndian, U32},
    transmute_ref,
};

use crate::{
    error::Error,
    filesystem::{exfat::probe_is_exfat, ntfs::probe_is_ntfs, vfat::probe_is_vfat},
    io::{BlockIo, Reader},
    partition::{
        PartAttributes, PartTableId, PartTableInfo, PartTableTag, PartTableType, Partition,
        PartitionId, PartitionType, aix::AIX_MAGIC,
    },
    probe::{Magic, ProbeFlags},
    std::fmt,
};

#[derive(Debug, Clone)]
pub enum MbrError {
    ProbablyAix,
    ProbablyGPT,
    ProbablyVFAT,
    ProbablyEXFAT,
    ProbablyNTFS,
    MissingBootIndicator,
    BadPrimaryExtendedOffset,
    MultipleExtendedPartitions,
    InvalidExtendedSignature,
    Overflow,
}

impl fmt::Display for MbrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MbrError::ProbablyAix => write!(f, "Partition table has AIX magic signature"),
            MbrError::ProbablyGPT => write!(f, "Partition table looks like GPT"),
            MbrError::ProbablyVFAT => write!(f, "Partition table looks like VFAT"),
            MbrError::ProbablyEXFAT => write!(f, "Partition table looks like EXFAT"),
            MbrError::ProbablyNTFS => write!(f, "Partition table looks like NTFS"),
            MbrError::MissingBootIndicator => {
                write!(f, "Missing boot indicator in partition entry")
            }
            MbrError::BadPrimaryExtendedOffset => {
                write!(f, "Bad offset in primary extended partition")
            }
            MbrError::MultipleExtendedPartitions => {
                write!(f, "Multiple extended partitions was found")
            }
            MbrError::InvalidExtendedSignature => {
                write!(f, "Extended partition is missing a valid signature")
            }
            MbrError::Overflow => {
                write!(f, "internal calculation overflowed")
            }
        }
    }
}

impl<E: fmt::Debug> From<MbrError> for Error<E> {
    fn from(e: MbrError) -> Self {
        Error::Mbr(e)
    }
}

const MBR_MAG: &[u8] = b"\x55\xAA";
const MBR_MAG_OFFSET: u64 = 510;

pub const MBR_MINSZ: Option<u64> = Some(512);
pub const MBR_MAGICS: Option<&'static [Magic]> = Some(&[Magic {
    magic: MBR_MAG,
    b_offset: MBR_MAG_OFFSET,
}]);

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout)]
pub struct MbrTable {
    pub boot_code1: [u8; 218],
    pub disk_timestamp: [u8; 6],
    pub boot_code2: [u8; 216],
    pub disk_id: [u8; 4],
    pub state: [u8; 2],
    pub partition_entries: [MbrPartitionEntry; 4],
    pub boot_signature: [u8; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct MbrPartitionEntry {
    pub boot_ind: u8,   /* 0x80 - active */
    pub begin_head: u8, /* begin CHS */
    pub begin_sector: u8,
    pub begin_cylinder: u8,
    pub sys_ind: MbrPartitionType, /* https://en.wikipedia.org/wiki/Partition_type */
    pub end_head: u8,              /* end CHS */
    pub end_sector: u8,
    pub end_cylinder: u8,
    pub start_sect: U32<LittleEndian>,
    pub nr_sects: U32<LittleEndian>,
}

impl MbrPartitionEntry {
    fn is_empty(&self) -> bool {
        Self::as_bytes(self) == [0u8; 16]
    }

    fn is_extended(&self) -> bool {
        self.sys_ind == MbrPartitionType::DOS_EXTENDED
            || self.sys_ind == MbrPartitionType::W95_EXTENDED
            || self.sys_ind == MbrPartitionType::LINUX_EXTENDED
    }

    fn flags(&self) -> MbrAttributes {
        MbrAttributes::from_bits_truncate(self.boot_ind)
    }
}

#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Ord,
    PartialOrd,
    Hash,
    FromBytes,
    IntoBytes,
    Unaligned,
    Immutable,
)]
pub struct MbrPartitionType(u8);

#[allow(dead_code)]
impl MbrPartitionType {
    pub const EMPTY: Self = Self(0x00);
    pub const FAT12: Self = Self(0x01);
    pub const XENIX_ROOT: Self = Self(0x02);
    pub const XENIX_USR: Self = Self(0x03);
    pub const FAT16_LESS32M: Self = Self(0x04);
    pub const DOS_EXTENDED: Self = Self(0x05);
    pub const FAT16: Self = Self(0x06);
    pub const HPFS_NTFS: Self = Self(0x07);
    pub const AIX: Self = Self(0x08);
    pub const AIX_BOOTABLE: Self = Self(0x09);
    pub const OS2_BOOTMNGR: Self = Self(0x0a);
    pub const W95_FAT32: Self = Self(0x0b);
    pub const W95_FAT32_LBA: Self = Self(0x0c);
    pub const W95_FAT16_LBA: Self = Self(0x0e);
    pub const W95_EXTENDED: Self = Self(0x0f);
    pub const OPUS: Self = Self(0x10);
    pub const HIDDEN_FAT12: Self = Self(0x11);
    pub const COMPAQ_DIAGNOSTICS: Self = Self(0x12);
    pub const HIDDEN_FAT16_L32M: Self = Self(0x14);
    pub const HIDDEN_FAT16: Self = Self(0x16);
    pub const HIDDEN_HPFS_NTFS: Self = Self(0x17);
    pub const AST_SMARTSLEEP: Self = Self(0x18);
    pub const HIDDEN_W95_FAT32: Self = Self(0x1b);
    pub const HIDDEN_W95_FAT32LBA: Self = Self(0x1c);
    pub const HIDDEN_W95_FAT16LBA: Self = Self(0x1e);
    pub const NEC_DOS: Self = Self(0x24);
    pub const PLAN9: Self = Self(0x39);
    pub const PARTITIONMAGIC: Self = Self(0x3c);
    pub const VENIX80286: Self = Self(0x40);
    pub const PPC_PREP_BOOT: Self = Self(0x41);
    pub const SFS: Self = Self(0x42);
    pub const QNX_4X: Self = Self(0x4d);
    pub const QNX_4X_2ND: Self = Self(0x4e);
    pub const QNX_4X_3RD: Self = Self(0x4f);
    pub const DM: Self = Self(0x50);
    pub const DM6_AUX1: Self = Self(0x51);
    pub const CPM: Self = Self(0x52);
    pub const DM6_AUX3: Self = Self(0x53);
    pub const DM6: Self = Self(0x54);
    pub const EZ_DRIVE: Self = Self(0x55);
    pub const GOLDEN_BOW: Self = Self(0x56);
    pub const PRIAM_EDISK: Self = Self(0x5c);
    pub const SPEEDSTOR: Self = Self(0x61);
    pub const GNU_HURD: Self = Self(0x63);
    pub const UNIXWARE: Self = Self(0x63);
    pub const NETWARE_286: Self = Self(0x64);
    pub const NETWARE_386: Self = Self(0x65);
    pub const DISKSECURE_MULTIBOOT: Self = Self(0x70);
    pub const PC_IX: Self = Self(0x75);
    pub const OLD_MINIX: Self = Self(0x80);
    pub const MINIX: Self = Self(0x81);
    pub const LINUX_SWAP: Self = Self(0x82);
    pub const SOLARIS_X86: Self = Self(0x82);
    pub const LINUX_DATA: Self = Self(0x83);
    pub const OS2_HIDDEN_DRIVE: Self = Self(0x84);
    pub const INTEL_HIBERNATION: Self = Self(0x84);
    pub const LINUX_EXTENDED: Self = Self(0x85);
    pub const NTFS_VOL_SET1: Self = Self(0x86);
    pub const NTFS_VOL_SET2: Self = Self(0x87);
    pub const LINUX_PLAINTEXT: Self = Self(0x88);
    pub const LINUX_LVM: Self = Self(0x8e);
    pub const AMOEBA: Self = Self(0x93);
    pub const AMOEBA_BBT: Self = Self(0x94);
    pub const BSD_OS: Self = Self(0x9f);
    pub const THINKPAD_HIBERNATION: Self = Self(0xa0);
    pub const FREEBSD: Self = Self(0xa5);
    pub const OPENBSD: Self = Self(0xa6);
    pub const NEXTSTEP: Self = Self(0xa7);
    pub const DARWIN_UFS: Self = Self(0xa8);
    pub const NETBSD: Self = Self(0xa9);
    pub const DARWIN_BOOT: Self = Self(0xab);
    pub const HFS_HFS: Self = Self(0xaf);
    pub const BSDI_FS: Self = Self(0xb7);
    pub const BSDI_SWAP: Self = Self(0xb8);
    pub const BOOTWIZARD_HIDDEN: Self = Self(0xbb);
    pub const ACRONIS_FAT32LBA: Self = Self(0xbc);
    pub const SOLARIS_BOOT: Self = Self(0xbe);
    pub const SOLARIS: Self = Self(0xbf);
    pub const DRDOS_FAT12: Self = Self(0xc1);
    pub const DRDOS_FAT16_L32M: Self = Self(0xc4);
    pub const DRDOS_FAT16: Self = Self(0xc6);
    pub const SYRINX: Self = Self(0xc7);
    pub const NONFS_DATA: Self = Self(0xda);
    pub const CPM_CTOS: Self = Self(0xdb);
    pub const DELL_UTILITY: Self = Self(0xde);
    pub const BOOTIT: Self = Self(0xdf);
    pub const DOS_ACCESS: Self = Self(0xe1);
    pub const DOS_RO: Self = Self(0xe3);
    pub const SPEEDSTOR_EXTENDED: Self = Self(0xe4);
    pub const RUFUS_EXTRA: Self = Self(0xea);
    pub const BEOS_FS: Self = Self(0xeb);
    pub const GPT: Self = Self(0xee);
    pub const EFI_SYSTEM: Self = Self(0xef);
    pub const LINUX_PARISC_BOOT: Self = Self(0xf0);
    pub const SPEEDSTOR1: Self = Self(0xf1);
    pub const SPEEDSTOR2: Self = Self(0xf4);
    pub const DOS_SECONDARY: Self = Self(0xf2);
    pub const EBBR_PROTECTIVE: Self = Self(0xf8);
    pub const VMWARE_VMFS: Self = Self(0xfb);
    pub const VMWARE_VMKCORE: Self = Self(0xfc);
    pub const LINUX_RAID: Self = Self(0xfd);
    pub const LANSTEP: Self = Self(0xfe);
    pub const XENIX_BBT: Self = Self(0xff);

    pub fn from_byte(byte: u8) -> Self {
        Self(byte)
    }

    pub fn as_byte(&self) -> u8 {
        self.0
    }
}

bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct MbrAttributes: u8 {
        const ACTIVE = 0x80;
        const INACTIVE = 0x00;
    }
}

fn is_valid_mbr<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    pt: &MbrTable,
) -> Result<(), Error<IO::Error>> {
    for entry in pt.partition_entries {
        let boot_ind = entry.flags();
        if !boot_ind.contains(MbrAttributes::INACTIVE) && !boot_ind.contains(MbrAttributes::ACTIVE)
        {
            return Err(MbrError::MissingBootIndicator.into());
        }

        if entry.sys_ind == MbrPartitionType::GPT {
            return Err(MbrError::ProbablyGPT.into());
        }
    }

    if probe_is_vfat(reader, offset).is_ok() {
        return Err(MbrError::ProbablyVFAT.into());
    }

    if probe_is_exfat(reader, offset)? {
        return Err(MbrError::ProbablyEXFAT.into());
    }

    if probe_is_ntfs(reader, offset)? {
        return Err(MbrError::ProbablyNTFS.into());
    }

    // TODO - is_lvm(pr) && is_empty_mbr(data)

    Ok(())
}

/// When `os_calls` is unavailable parsing will default to 512 byte logical
/// sector size as MBR does not provide enough information to figure out the
/// partition table sector size from its header content alone.
///
/// When `os_calls` is available parsing will use the disks logical sector size
/// for calculations.
pub fn probe_mbr<IO: BlockIo>(
    reader: &mut Reader<IO>,
    _: ProbeFlags,
    offset: u64,
    _: Magic,
) -> Result<PartTableInfo, Error<IO::Error>> {
    let buf: [u8; size_of::<MbrTable>()] = reader.read_exact_at(offset)?;

    if buf[0..3] == AIX_MAGIC {
        return Err(MbrError::ProbablyAix.into());
    }

    let mbr_pt: &MbrTable = transmute_ref!(&buf);

    is_valid_mbr(reader, offset, mbr_pt)?;

    #[cfg(feature = "os_calls")]
    let ssz = reader.logical_sector_size()?;
    #[cfg(not(feature = "os_calls"))]
    const ssz: u64 = 512;

    let mut partitions: Vec<Partition> = Vec::new();

    let primary = mbr_pt.partition_entries;
    let mut part_no: u8 = 1;
    let mut extended: Option<MbrPartitionEntry> = None;

    for part in primary {
        let start = u64::from(part.start_sect)
            .checked_mul(ssz)
            .ok_or(MbrError::Overflow)?;

        let size = u64::from(part.nr_sects)
            .checked_mul(ssz)
            .ok_or(MbrError::Overflow)?;

        if size == 0 {
            part_no += 1;
            continue;
        }

        if part.is_extended() {
            if extended.is_none() {
                extended = Some(part);
            } else {
                return Err(MbrError::MultipleExtendedPartitions.into());
            }
            continue;
        }

        partitions.push(Partition {
            start,
            end: start.checked_add(start).ok_or(MbrError::Overflow)?,
            partition_id: PartitionId::Mbr {
                disk: u32::from_le_bytes(mbr_pt.disk_id),
                part_no,
            },
            partition_type: PartitionType::Mbr(part.sys_ind),
            part_no: u64::from(part_no),
            partition_name: None,
            attributes: PartAttributes::Mbr(part.boot_ind),
        });
    }

    if let Some(extend) = extended {
        todo!()
    }

    let mut info = PartTableInfo::new();

    info.set(PartTableTag::PartTableType(PartTableType::Mbr));
    info.set(PartTableTag::PartTableId(PartTableId::Mbr {
        disk: u32::from_le_bytes(mbr_pt.disk_id),
    }));
    info.set(PartTableTag::Magic(MBR_MAG.to_vec()));
    info.set(PartTableTag::MagicOffset(MBR_MAG_OFFSET));
    if !partitions.is_empty() {
        info.set(PartTableTag::Partitions(partitions));
    }

    Ok(info)
}
