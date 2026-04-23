use bitflags::bitflags;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned,
    byteorder::{LittleEndian, U32},
    transmute_ref,
};

use crate::{
    error::Error,
    filesystem::{exfat::probe_is_exfat, vfat::probe_is_vfat},
    io::{BlockIo, Reader},
    partition::aix::AIX_MAGIC,
    probe::{Magic, PartTableInfo},
    std::fmt,
};

#[derive(Debug)]
pub enum MbrError {
    ProbablyAix,
    ProbablyGPT,
    ProbablyVFAT,
    ProbablyEXFAT,
    ProbablyNTFS,
    MissingBootIndicator,
    BadPrimaryExtendedOffset,
    InvalidExtendedSignature,
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
            MbrError::InvalidExtendedSignature => {
                write!(f, "Extended partition is missing a valid signature")
            }
        }
    }
}

impl<IO: BlockIo> From<MbrError> for Error<IO> {
    fn from(e: MbrError) -> Self {
        Error::Mbr(e)
    }
}

pub const MBR_MAGICS: Option<&'static [Magic]> = Some(&[Magic {
    magic: b"\x55\xAA",
    len: 2,
    b_offset: 510,
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
        self.sys_ind == MbrPartitionType::MBR_DOS_EXTENDED_PARTITION
            || self.sys_ind == MbrPartitionType::MBR_W95_EXTENDED_PARTITION
            || self.sys_ind == MbrPartitionType::MBR_LINUX_EXTENDED_PARTITION
    }

    fn flags(&self) -> MbrAttributes {
        MbrAttributes::from_bits_truncate(self.boot_ind)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Unaligned, Immutable)]
pub struct MbrPartitionType(u8);

#[allow(dead_code)]
impl MbrPartitionType {
    pub const MBR_EMPTY_PARTITION: Self = Self(0x00);
    pub const MBR_FAT12_PARTITION: Self = Self(0x01);
    pub const MBR_XENIX_ROOT_PARTITION: Self = Self(0x02);
    pub const MBR_XENIX_USR_PARTITION: Self = Self(0x03);
    pub const MBR_FAT16_LESS32M_PARTITION: Self = Self(0x04);
    pub const MBR_DOS_EXTENDED_PARTITION: Self = Self(0x05);
    pub const MBR_FAT16_PARTITION: Self = Self(0x06);
    pub const MBR_HPFS_NTFS_PARTITION: Self = Self(0x07);
    pub const MBR_AIX_PARTITION: Self = Self(0x08);
    pub const MBR_AIX_BOOTABLE_PARTITION: Self = Self(0x09);
    pub const MBR_OS2_BOOTMNGR_PARTITION: Self = Self(0x0a);
    pub const MBR_W95_FAT32_PARTITION: Self = Self(0x0b);
    pub const MBR_W95_FAT32_LBA_PARTITION: Self = Self(0x0c);
    pub const MBR_W95_FAT16_LBA_PARTITION: Self = Self(0x0e);
    pub const MBR_W95_EXTENDED_PARTITION: Self = Self(0x0f);
    pub const MBR_OPUS_PARTITION: Self = Self(0x10);
    pub const MBR_HIDDEN_FAT12_PARTITION: Self = Self(0x11);
    pub const MBR_COMPAQ_DIAGNOSTICS_PARTITION: Self = Self(0x12);
    pub const MBR_HIDDEN_FAT16_L32M_PARTITION: Self = Self(0x14);
    pub const MBR_HIDDEN_FAT16_PARTITION: Self = Self(0x16);
    pub const MBR_HIDDEN_HPFS_NTFS_PARTITION: Self = Self(0x17);
    pub const MBR_AST_SMARTSLEEP_PARTITION: Self = Self(0x18);
    pub const MBR_HIDDEN_W95_FAT32_PARTITION: Self = Self(0x1b);
    pub const MBR_HIDDEN_W95_FAT32LBA_PARTITION: Self = Self(0x1c);
    pub const MBR_HIDDEN_W95_FAT16LBA_PARTITION: Self = Self(0x1e);
    pub const MBR_NEC_DOS_PARTITION: Self = Self(0x24);
    pub const MBR_PLAN9_PARTITION: Self = Self(0x39);
    pub const MBR_PARTITIONMAGIC_PARTITION: Self = Self(0x3c);
    pub const MBR_VENIX80286_PARTITION: Self = Self(0x40);
    pub const MBR_PPC_PREP_BOOT_PARTITION: Self = Self(0x41);
    pub const MBR_SFS_PARTITION: Self = Self(0x42);
    pub const MBR_QNX_4X_PARTITION: Self = Self(0x4d);
    pub const MBR_QNX_4X_2ND_PARTITION: Self = Self(0x4e);
    pub const MBR_QNX_4X_3RD_PARTITION: Self = Self(0x4f);
    pub const MBR_DM_PARTITION: Self = Self(0x50);
    pub const MBR_DM6_AUX1_PARTITION: Self = Self(0x51);
    pub const MBR_CPM_PARTITION: Self = Self(0x52);
    pub const MBR_DM6_AUX3_PARTITION: Self = Self(0x53);
    pub const MBR_DM6_PARTITION: Self = Self(0x54);
    pub const MBR_EZ_DRIVE_PARTITION: Self = Self(0x55);
    pub const MBR_GOLDEN_BOW_PARTITION: Self = Self(0x56);
    pub const MBR_PRIAM_EDISK_PARTITION: Self = Self(0x5c);
    pub const MBR_SPEEDSTOR_PARTITION: Self = Self(0x61);
    pub const MBR_GNU_HURD_PARTITION: Self = Self(0x63);
    pub const MBR_UNIXWARE_PARTITION: Self = Self(0x63);
    pub const MBR_NETWARE_286_PARTITION: Self = Self(0x64);
    pub const MBR_NETWARE_386_PARTITION: Self = Self(0x65);
    pub const MBR_DISKSECURE_MULTIBOOT_PARTITION: Self = Self(0x70);
    pub const MBR_PC_IX_PARTITION: Self = Self(0x75);
    pub const MBR_OLD_MINIX_PARTITION: Self = Self(0x80);
    pub const MBR_MINIX_PARTITION: Self = Self(0x81);
    pub const MBR_LINUX_SWAP_PARTITION: Self = Self(0x82);
    pub const MBR_SOLARIS_X86_PARTITION: Self = Self(0x82);
    pub const MBR_LINUX_DATA_PARTITION: Self = Self(0x83);
    pub const MBR_OS2_HIDDEN_DRIVE_PARTITION: Self = Self(0x84);
    pub const MBR_INTEL_HIBERNATION_PARTITION: Self = Self(0x84);
    pub const MBR_LINUX_EXTENDED_PARTITION: Self = Self(0x85);
    pub const MBR_NTFS_VOL_SET1_PARTITION: Self = Self(0x86);
    pub const MBR_NTFS_VOL_SET2_PARTITION: Self = Self(0x87);
    pub const MBR_LINUX_PLAINTEXT_PARTITION: Self = Self(0x88);
    pub const MBR_LINUX_LVM_PARTITION: Self = Self(0x8e);
    pub const MBR_AMOEBA_PARTITION: Self = Self(0x93);
    pub const MBR_AMOEBA_BBT_PARTITION: Self = Self(0x94);
    pub const MBR_BSD_OS_PARTITION: Self = Self(0x9f);
    pub const MBR_THINKPAD_HIBERNATION_PARTITION: Self = Self(0xa0);
    pub const MBR_FREEBSD_PARTITION: Self = Self(0xa5);
    pub const MBR_OPENBSD_PARTITION: Self = Self(0xa6);
    pub const MBR_NEXTSTEP_PARTITION: Self = Self(0xa7);
    pub const MBR_DARWIN_UFS_PARTITION: Self = Self(0xa8);
    pub const MBR_NETBSD_PARTITION: Self = Self(0xa9);
    pub const MBR_DARWIN_BOOT_PARTITION: Self = Self(0xab);
    pub const MBR_HFS_HFS_PARTITION: Self = Self(0xaf);
    pub const MBR_BSDI_FS_PARTITION: Self = Self(0xb7);
    pub const MBR_BSDI_SWAP_PARTITION: Self = Self(0xb8);
    pub const MBR_BOOTWIZARD_HIDDEN_PARTITION: Self = Self(0xbb);
    pub const MBR_ACRONIS_FAT32LBA_PARTITION: Self = Self(0xbc);
    pub const MBR_SOLARIS_BOOT_PARTITION: Self = Self(0xbe);
    pub const MBR_SOLARIS_PARTITION: Self = Self(0xbf);
    pub const MBR_DRDOS_FAT12_PARTITION: Self = Self(0xc1);
    pub const MBR_DRDOS_FAT16_L32M_PARTITION: Self = Self(0xc4);
    pub const MBR_DRDOS_FAT16_PARTITION: Self = Self(0xc6);
    pub const MBR_SYRINX_PARTITION: Self = Self(0xc7);
    pub const MBR_NONFS_DATA_PARTITION: Self = Self(0xda);
    pub const MBR_CPM_CTOS_PARTITION: Self = Self(0xdb);
    pub const MBR_DELL_UTILITY_PARTITION: Self = Self(0xde);
    pub const MBR_BOOTIT_PARTITION: Self = Self(0xdf);
    pub const MBR_DOS_ACCESS_PARTITION: Self = Self(0xe1);
    pub const MBR_DOS_RO_PARTITION: Self = Self(0xe3);
    pub const MBR_SPEEDSTOR_EXTENDED_PARTITION: Self = Self(0xe4);
    pub const MBR_RUFUS_EXTRA_PARTITION: Self = Self(0xea);
    pub const MBR_BEOS_FS_PARTITION: Self = Self(0xeb);
    pub const MBR_GPT_PARTITION: Self = Self(0xee);
    pub const MBR_EFI_SYSTEM_PARTITION: Self = Self(0xef);
    pub const MBR_LINUX_PARISC_BOOT_PARTITION: Self = Self(0xf0);
    pub const MBR_SPEEDSTOR1_PARTITION: Self = Self(0xf1);
    pub const MBR_SPEEDSTOR2_PARTITION: Self = Self(0xf4);
    pub const MBR_DOS_SECONDARY_PARTITION: Self = Self(0xf2);
    pub const MBR_EBBR_PROTECTIVE_PARTITION: Self = Self(0xf8);
    pub const MBR_VMWARE_VMFS_PARTITION: Self = Self(0xfb);
    pub const MBR_VMWARE_VMKCORE_PARTITION: Self = Self(0xfc);
    pub const MBR_LINUX_RAID_PARTITION: Self = Self(0xfd);
    pub const MBR_LANSTEP_PARTITION: Self = Self(0xfe);
    pub const MBR_XENIX_BBT_PARTITION: Self = Self(0xff);

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
) -> Result<(), Error<IO>> {
    for entry in pt.partition_entries {
        let boot_ind = entry.flags();
        if !boot_ind.contains(MbrAttributes::INACTIVE) && !boot_ind.contains(MbrAttributes::ACTIVE)
        {
            return Err(MbrError::MissingBootIndicator.into());
        }

        if entry.sys_ind == MbrPartitionType::MBR_GPT_PARTITION {
            return Err(MbrError::ProbablyGPT.into());
        }
    }

    if probe_is_vfat(reader, offset).is_err() {
        return Err(MbrError::ProbablyVFAT.into());
    }

    if probe_is_exfat(reader, offset).is_err() {
        return Err(MbrError::ProbablyEXFAT.into());
    }

    // if probe_is_ntfs(probe).is_err() {
    //     return Err(MbrError::ProbablyNTFS);
    // }

    // TODO - is_lvm(pr) && is_empty_mbr(data)

    Ok(())
}

pub fn probe_mbr<IO: BlockIo>(
    reader: &mut Reader<IO>,
    offset: u64,
    mag: Magic,
) -> Result<PartTableInfo, Error<IO>> {
    let buf: [u8; size_of::<MbrTable>()] = reader.read_exact_at(offset).map_err(Error::io)?;

    let mbr_pt: &MbrTable = transmute_ref!(&buf);

    if mbr_pt.boot_code1[0..3] == AIX_MAGIC {
        return Err(MbrError::ProbablyAix.into());
    }

    is_valid_mbr(reader, offset, mbr_pt)?;

    let primary = mbr_pt.partition_entries;

    todo!()
}
