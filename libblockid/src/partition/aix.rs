use crate::{
    error::Error,
    io::{BlockIo, Reader},
    partition::{PtInfo, PtTag, PtType},
    probe::{Magic, ProbeFlags},
};

pub const AIX_MAGIC: [u8; 4] = [0xC9, 0xC2, 0xD4, 0xC1];

#[derive(Debug, Clone)]
pub enum AixError {}

impl<E: core::fmt::Debug> From<AixError> for Error<E> {
    fn from(e: AixError) -> Self {
        Error::Aix(e)
    }
}

pub const AIX_MINSZ: Option<u64> = None;
pub const AIX_MAGICS: Option<&'static [Magic]> = Some(&[Magic {
    magic: &AIX_MAGIC,
    b_offset: 0,
}]);

pub fn probe_aix<IO: BlockIo>(
    _: &mut Reader<IO>,
    _: ProbeFlags,
    _: u64,
    _: Magic,
) -> Result<PtInfo, Error<IO::Error>> {
    let mut info = PtInfo::new();

    info.set(PtTag::PtType(PtType::Aix));
    info.set(PtTag::Magic(AIX_MAGIC.to_vec()));
    info.set(PtTag::MagicOffset(0));

    return Ok(info);
}
