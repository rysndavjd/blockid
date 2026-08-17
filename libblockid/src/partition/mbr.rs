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
        Partition, PartitionAttributes, PartitionId, PartitionType, PtInfo, PtType, aix::AIX_MAGIC,
    },
    probe::Magic,
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
    bytes: MBR_MAG,
    offset: MBR_MAG_OFFSET,
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
    pub boot_ind: MbrAttributes, /* 0x80 - active */
    pub begin_head: u8,          /* begin CHS */
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
    #![allow(dead_code)]
    fn is_empty(&self) -> bool {
        self.as_bytes() == [0u8; 16]
    }

    fn is_extended(&self) -> bool {
        self.sys_ind == MbrPartitionType::DOS_EXTENDED
            || self.sys_ind == MbrPartitionType::W95_EXTENDED
            || self.sys_ind == MbrPartitionType::LINUX_EXTENDED
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

bitflags! {
    impl MbrPartitionType: u8 {
    const EMPTY = 0x00;
    const FAT12 = 0x01;
    const XENIX_ROOT = 0x02;
    const XENIX_USR = 0x03;
    const FAT16_LESS32M = 0x04;
    const DOS_EXTENDED = 0x05;
    const FAT16 = 0x06;
    const HPFS_NTFS = 0x07;
    const AIX = 0x08;
    const AIX_BOOTABLE = 0x09;
    const OS2_BOOTMNGR = 0x0a;
    const W95_FAT32 = 0x0b;
    const W95_FAT32_LBA = 0x0c;
    const W95_FAT16_LBA = 0x0e;
    const W95_EXTENDED = 0x0f;
    const OPUS = 0x10;
    const HIDDEN_FAT12 = 0x11;
    const COMPAQ_DIAGNOSTICS = 0x12;
    const HIDDEN_FAT16_L32M = 0x14;
    const HIDDEN_FAT16 = 0x16;
    const HIDDEN_HPFS_NTFS = 0x17;
    const AST_SMARTSLEEP = 0x18;
    const HIDDEN_W95_FAT32 = 0x1b;
    const HIDDEN_W95_FAT32LBA = 0x1c;
    const HIDDEN_W95_FAT16LBA = 0x1e;
    const NEC_DOS = 0x24;
    const PLAN9 = 0x39;
    const PARTITIONMAGIC = 0x3c;
    const VENIX80286 = 0x40;
    const PPC_PREP_BOOT = 0x41;
    const SFS = 0x42;
    const QNX_4X = 0x4d;
    const QNX_4X_2ND = 0x4e;
    const QNX_4X_3RD = 0x4f;
    const DM = 0x50;
    const DM6_AUX1 = 0x51;
    const CPM = 0x52;
    const DM6_AUX3 = 0x53;
    const DM6 = 0x54;
    const EZ_DRIVE = 0x55;
    const GOLDEN_BOW = 0x56;
    const PRIAM_EDISK = 0x5c;
    const SPEEDSTOR = 0x61;
    const GNU_HURD = 0x63;
    const UNIXWARE = 0x63;
    const NETWARE_286 = 0x64;
    const NETWARE_386 = 0x65;
    const DISKSECURE_MULTIBOOT = 0x70;
    const PC_IX = 0x75;
    const OLD_MINIX = 0x80;
    const MINIX = 0x81;
    const LINUX_SWAP = 0x82;
    const SOLARIS_X86 = 0x82;
    const LINUX_DATA = 0x83;
    const OS2_HIDDEN_DRIVE = 0x84;
    const INTEL_HIBERNATION = 0x84;
    const LINUX_EXTENDED = 0x85;
    const NTFS_VOL_SET1 = 0x86;
    const NTFS_VOL_SET2 = 0x87;
    const LINUX_PLAINTEXT = 0x88;
    const LINUX_LVM = 0x8e;
    const AMOEBA = 0x93;
    const AMOEBA_BBT = 0x94;
    const BSD_OS = 0x9f;
    const THINKPAD_HIBERNATION = 0xa0;
    const FREEBSD = 0xa5;
    const OPENBSD = 0xa6;
    const NEXTSTEP = 0xa7;
    const DARWIN_UFS = 0xa8;
    const NETBSD = 0xa9;
    const DARWIN_BOOT = 0xab;
    const HFS_HFS = 0xaf;
    const BSDI_FS = 0xb7;
    const BSDI_SWAP = 0xb8;
    const BOOTWIZARD_HIDDEN = 0xbb;
    const ACRONIS_FAT32LBA = 0xbc;
    const SOLARIS_BOOT = 0xbe;
    const SOLARIS = 0xbf;
    const DRDOS_FAT12 = 0xc1;
    const DRDOS_FAT16_L32M = 0xc4;
    const DRDOS_FAT16 = 0xc6;
    const SYRINX = 0xc7;
    const NONFS_DATA = 0xda;
    const CPM_CTOS = 0xdb;
    const DELL_UTILITY = 0xde;
    const BOOTIT = 0xdf;
    const DOS_ACCESS = 0xe1;
    const DOS_RO = 0xe3;
    const SPEEDSTOR_EXTENDED = 0xe4;
    const RUFUS_EXTRA = 0xea;
    const BEOS_FS = 0xeb;
    const GPT = 0xee;
    const EFI_SYSTEM = 0xef;
    const LINUX_PARISC_BOOT = 0xf0;
    const SPEEDSTOR1 = 0xf1;
    const SPEEDSTOR2 = 0xf4;
    const DOS_SECONDARY = 0xf2;
    const EBBR_PROTECTIVE = 0xf8;
    const VMWARE_VMFS = 0xfb;
    const VMWARE_VMKCORE = 0xfc;
    const LINUX_RAID = 0xfd;
    const LANSTEP = 0xfe;
    const XENIX_BBT = 0xff;
    }
}

#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    FromBytes,
    IntoBytes,
    Unaligned,
    Immutable,
    KnownLayout,
)]
pub struct MbrAttributes(u8);

bitflags! {
    impl MbrAttributes: u8 {
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
        if !entry
            .boot_ind
            .contains(MbrAttributes::INACTIVE | MbrAttributes::ACTIVE)
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
    offset: u64,
    _: Magic,
) -> Result<PtInfo, Error<IO::Error>> {
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
    let mut part_no: u8 = 1;
    let mut extended: Option<MbrPartitionEntry> = None;

    for part in mbr_pt.partition_entries {
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
            attributes: PartitionAttributes::Mbr(part.boot_ind),
        });
    }

    if let Some(extend) = extended {
        todo!()
    }

    let mut info = PtInfo::empty();

    info.set_pt_type(PtType::Mbr);
    info.set_pt_id(u32::from_le_bytes(mbr_pt.disk_id).into());
    info.set_magic(MBR_MAG.to_vec(), MBR_MAG_OFFSET);
    if !partitions.is_empty() {
        info.set_partitions(partitions);
    }

    Ok(info)
}
