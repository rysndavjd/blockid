use crate::std::fmt;
pub use crate::{
    filesystem::{
        apfs::ApfsError, cramfs::CramfsError, exfat::ExFatError, ext::ExtError, luks::LuksError,
        ntfs::NtfsError, vfat::VFatError, vxfs::VxfsError, xfs::XfsError,
    },
    partition::{aix::AixError, gpt::GptError, mbr::MbrError},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuilderError {}

/// Main error type returned by probing operations.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum Error<E: fmt::Debug> {
    /// An IO error from the underlying reader.
    Io(E),
    /// Errors returned from APFS (Apple File System) probing logic.
    Apfs(ApfsError),
    /// Errors returned from LUKS (Linux Unified Key Setup) probing logic.
    Luks(LuksError),
    /// Errors returned from ExFAT probing logic.
    ExFat(ExFatError),
    /// Errors returned from EXT2/3/4 probing logic.
    Ext(ExtError),
    /// Errors returned from NTFS (NT File System) probing logic.
    Ntfs(NtfsError),
    /// Errors returned from VFAT probing logic.
    VFat(VFatError),
    /// Errors returned from VXFS (Veritas File System) probing logic.
    Vxfs(VxfsError),
    /// Errors returned from XFS probing logic.
    Xfs(XfsError),
    /// Errors returned from cramfs probeing logic
    Cramfs(CramfsError),
    /// Errors returned from AIX probing logic.
    Aix(AixError),
    /// Errors returned from MBR (Master Boot Record) probing logic.
    Mbr(MbrError),
    /// Errors returned from GPT (GUID Partition Table) probing logic.
    Gpt(GptError),
    /// No magic signature was found at any expected offset.
    UnableToLocateMagicSignature,
    /// The device is smaller than the minimum required to hold
    /// the supported filesystem or partition table structure.
    DeviceTooSmall,
    /// Range end exceeds given size read
    RangeEndExceedsGivenSize,
    /// The provided offset exceeds the bounds of the device.
    OffsetExceedsDeviceSize,
    /// All available probes were attempted and none succeeded.
    ProbesExhausted,
}

impl<E: fmt::Debug> fmt::Display for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO Error: {:?}", e),
            Self::Apfs(e) => write!(f, "APFS Error: {}", e),
            Self::Luks(e) => write!(f, "LUKS Error: {}", e),
            Self::ExFat(e) => write!(f, "exFAT Error: {}", e),
            Self::Ext(e) => write!(f, "Ext Error: {}", e),
            Self::Ntfs(e) => write!(f, "NTFS Error: {}", e),
            Self::VFat(e) => write!(f, "VFAT Error: {}", e),
            Self::Vxfs(_) => write!(f, "VXFS Error"),
            Self::Xfs(e) => write!(f, "XFS Error: {}", e),
            Self::Cramfs(e) => write!(f, "cramfs error: {}", e),
            Self::Aix(_) => write!(f, "AIX Error"),
            Self::Mbr(e) => write!(f, "MBR Error: {}", e),
            Self::Gpt(e) => write!(f, "GPT Error: {}", e),
            Self::UnableToLocateMagicSignature => write!(f, "unable to locate magic signature"),
            Self::DeviceTooSmall => write!(
                f,
                "device is smaller than the minimum required to hold the supported filesystem or partition table structure"
            ),
            Self::RangeEndExceedsGivenSize => {
                write!(f, "range end exceeds given size read")
            }
            Self::OffsetExceedsDeviceSize => {
                write!(f, "provided offset exceeds the bounds of the device")
            }
            Self::ProbesExhausted => {
                write!(f, "all available probes were attempted and none succeeded")
            }
        }
    }
}
