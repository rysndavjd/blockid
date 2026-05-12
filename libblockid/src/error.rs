pub use crate::{
    filesystem::{
        apfs::ApfsError, exfat::ExFatError, ext::ExtError, luks::LuksError, ntfs::NtfsError,
        vfat::VFatError, vxfs::VxfsError, xfs::XfsError,
    },
    partition::{gpt::GptError, mbr::MbrError},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuilderError {}

/// Main error type returned by probing operations.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum Error<E: crate::std::fmt::Debug> {
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
    /// Errors returned from MBR (Master Boot Record) probing logic.
    Mbr(MbrError),
    /// Errors returned from GPT (GUID Partition Table) probing logic.
    Gpt(GptError),
    /// No magic signature was found at any expected offset.
    UnableToLocateMagicSignature,
    /// The device is smaller than the minimum required to hold
    /// the supported filesystem or partition table structure.
    DeviceTooSmall,
    /// All available probes were attempted and none succeeded.
    ProbesExhausted,
}
