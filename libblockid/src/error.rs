use crate::{
    filesystem::{
        apfs::ApfsError, exfat::ExFatError, ext::ExtError, luks::LuksError, ntfs::NtfsError,
        vfat::VFatError, xfs::XfsError,
    },
    partition::{gpt::GptError, mbr::MbrError},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuilderError {}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum Error<E: crate::std::fmt::Debug> {
    Io(E),
    Apfs(ApfsError),
    Luks(LuksError),
    ExFat(ExFatError),
    Ext(ExtError),
    Ntfs(NtfsError),
    VFat(VFatError),
    Xfs(XfsError),
    Mbr(MbrError),
    Gpt(GptError),
    UnableToLocateMagicSignature,
    DeviceTooSmall,
    ProbesExhausted,
}
