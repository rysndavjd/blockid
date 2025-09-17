pub mod apfs;
pub mod exfat;
pub mod ext;
pub mod linux_swap;
pub mod ntfs;
pub mod vfat;
pub mod volume_id;
pub mod xfs;

use thiserror::Error;

use crate::filesystems::{
    apfs::ApfsError, exfat::ExFatError, ext::ExtError, linux_swap::SwapError, ntfs::NtfsError,
    vfat::FatError, xfs::XfsError,
};

#[derive(Debug, Error)]
pub enum FsError {
    #[error("EXFAT filesystem error: {0}")]
    ExfatError(#[from] ExFatError),
    #[error("EXT filesystem error: {0}")]
    ExtError(#[from] ExtError),
    #[error("Linux Swap filesystem error: {0}")]
    LinuxSwap(#[from] SwapError),
    #[error("NTFS filesystem error: {0}")]
    Ntfs(#[from] NtfsError),
    #[error("VFAT filesystem error: {0}")]
    Vfat(#[from] FatError),
    #[error("XFS filesystem error: {0}")]
    Xfs(#[from] XfsError),
    #[error("APFS filesystem error: {0}")]
    ApfsError(#[from] ApfsError),
}
