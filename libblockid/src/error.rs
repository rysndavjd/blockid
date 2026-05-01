use std::fmt::Debug;

use crate::{
    filesystem::{exfat::ExFatError, ext::ExtError, luks::LuksError, vfat::VFatError},
    partition::{gpt::GptError, mbr::MbrError},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuilderError {}

#[non_exhaustive]
#[derive(Debug)]
pub enum Error<E: Debug> {
    Io(E),
    Luks(LuksError),
    ExFat(ExFatError),
    Ext(ExtError),
    VFat(VFatError),
    Mbr(MbrError),
    Gpt(GptError),
    ProbesExhausted,
}

// #[derive(Clone, Copy, Debug, Eq, PartialEq)]

// impl crate::std::error::Error for Error {}
