use uuid::Uuid;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, LittleEndian, Unaligned,
    byteorder::{BigEndian, U16, U32, U64},
    transmute_ref,
};

use crate::{
    BlockTag, BlockType, Id, ProbeFlags, Usage,
    error::Error,
    filesystem::BlockInfo,
    io::{BlockIo, Reader},
    probe::Magic,
    std::{fmt, mem::offset_of, str::Utf8Error},
    util::{decode_utf8_from, decode_utf8_lossy_from},
};

#[derive(Debug, Clone)]
pub enum XfsError {
    Utf8Error(Utf8Error),
    InvalidHeaderRanges,
    InvalidHeaderVersion,
    InvalidHeaderFeatures,
    HeaderChecksumInvalid,
}

impl fmt::Display for XfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XfsError::Utf8Error(e) => write!(f, "Filesystem label contains invalid UTF-8: {e}"),
            XfsError::InvalidHeaderRanges => write!(f, "Invalid XFS header ranges"),
            XfsError::InvalidHeaderVersion => write!(f, "Invalid XFS header version number"),
            XfsError::InvalidHeaderFeatures => write!(f, "Invalid XFS header features"),
            XfsError::HeaderChecksumInvalid => write!(f, "Invalid header checksum"),
        }
    }
}

impl<E: fmt::Debug> From<XfsError> for Error<E> {
    fn from(e: XfsError) -> Self {
        Error::Xfs(e)
    }
}

pub const XFS_MINSZ: Option<u64> = None;
pub const XFS_MAGICS: Option<&'static [Magic]> = Some(&[Magic {
    magic: b"XFSB",
    len: 4,
    b_offset: 0,
}]);

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout)]
pub struct XfsSuperBlock {
    magicnum: U32<BigEndian>,
    blocksize: U32<BigEndian>,
    dblocks: U64<BigEndian>,
    rblocks: U64<BigEndian>,
    rextents: U64<BigEndian>,
    uuid: [u8; 16],
    logstart: U64<BigEndian>,
    rootino: U64<BigEndian>,
    rbmino: U64<BigEndian>,
    rsumino: U64<BigEndian>,
    rextsize: U32<BigEndian>,
    agblocks: U32<BigEndian>,
    agcount: U32<BigEndian>,
    rbmblocks: U32<BigEndian>,
    logblocks: U32<BigEndian>,

    versionnum: U16<BigEndian>,
    sectsize: U16<BigEndian>,
    inodesize: U16<BigEndian>,
    inopblock: U16<BigEndian>,
    fname: [u8; 12],
    blocklog: u8,
    sectlog: u8,
    inodelog: u8,
    inopblog: u8,
    agblklog: u8,
    rextslog: u8,
    inprogress: u8,
    imax_pct: u8,

    icount: U64<BigEndian>,
    ifree: U64<BigEndian>,
    fdblocks: U64<BigEndian>,
    frextents: U64<BigEndian>,
    uquotino: U64<BigEndian>,
    gquotino: U64<BigEndian>,
    qflags: U16<BigEndian>,
    flags: u8,
    shared_vn: u8,
    inoalignmt: U32<BigEndian>,
    unit: U32<BigEndian>,
    width: U32<BigEndian>,
    dirblklog: u8,
    logsectlog: u8,
    logsectsize: U16<BigEndian>,
    logsunit: U32<BigEndian>,
    features2: U32<BigEndian>,
    bad_features2: U32<BigEndian>,

    features_compat: U32<BigEndian>,
    features_ro_compat: U32<BigEndian>,
    features_incompat: U32<BigEndian>,
    features_log_incompat: U32<BigEndian>,
    crc: U32<LittleEndian>,
    spino_align: U32<BigEndian>,
    pquotino: U64<BigEndian>,
    lsn: U64<BigEndian>,
    meta_uuid: [u8; 16],
    rrmapino: U64<BigEndian>,
}

impl XfsSuperBlock {
    const MIN_BLOCKSIZE_LOG: u8 = 9;
    const MAX_BLOCKSIZE_LOG: u8 = 16;
    const MIN_BLOCKSIZE: u32 = 1 << XfsSuperBlock::MIN_BLOCKSIZE_LOG;
    const MAX_BLOCKSIZE: u32 = 1 << XfsSuperBlock::MAX_BLOCKSIZE_LOG;
    const MIN_SECTORSIZE_LOG: u8 = 9;
    const MAX_SECTORSIZE_LOG: u8 = 15;
    const MIN_SECTORSIZE: u16 = 1 << XfsSuperBlock::MIN_SECTORSIZE_LOG;
    const MAX_SECTORSIZE: u16 = 1 << XfsSuperBlock::MAX_SECTORSIZE_LOG;
    const DINODE_MIN_LOG: u8 = 8;
    const DINODE_MAX_LOG: u8 = 11;
    const DINODE_MIN_SIZE: u16 = 1 << XfsSuperBlock::DINODE_MIN_LOG;
    const DINODE_MAX_SIZE: u16 = 1 << XfsSuperBlock::DINODE_MAX_LOG;

    const MAX_RTEXTSIZE: u32 = 1024 * 1024 * 1024;
    //const DFL_RTEXTSIZE: u32 = 64 * 1024;
    const MIN_RTEXTSIZE: u32 = 4 * 1024;
    const MIN_AG_BLOCKS: u64 = 64;

    fn max_dblocks(&self) -> u64 {
        u64::from(self.agcount) * u64::from(self.agblocks)
    }

    fn min_dblocks(&self) -> u64 {
        (u64::from(self.agcount) - 1) * (u64::from(self.agblocks) + XfsSuperBlock::MIN_AG_BLOCKS)
    }

    const SB_VERSION_MOREBITSBIT: u16 = 0x8000;
    const SB_VERSION2_CRCBIT: u32 = 0x00000100;

    pub fn verify(&self, crc_area: &mut [u8]) -> Result<(), XfsError> {
        if self.agcount.get() == 0
            || self.sectsize.get() < XfsSuperBlock::MIN_SECTORSIZE
            || self.sectsize.get() > XfsSuperBlock::MAX_SECTORSIZE
            || self.sectlog < XfsSuperBlock::MIN_SECTORSIZE_LOG
            || self.sectlog > XfsSuperBlock::MAX_SECTORSIZE_LOG
            || self.sectsize.get() != (1 << self.sectlog)
            || self.blocksize.get() < XfsSuperBlock::MIN_BLOCKSIZE
            || self.blocksize.get() > XfsSuperBlock::MAX_BLOCKSIZE
            || self.blocklog < XfsSuperBlock::MIN_BLOCKSIZE_LOG
            || self.blocklog > XfsSuperBlock::MAX_BLOCKSIZE_LOG
            || self.blocksize.get() != (1 << self.blocklog)
            || self.inodesize.get() < XfsSuperBlock::DINODE_MIN_SIZE
            || self.inodesize.get() > XfsSuperBlock::DINODE_MAX_SIZE
            || self.inodelog < XfsSuperBlock::DINODE_MIN_LOG
            || self.inodelog > XfsSuperBlock::DINODE_MAX_LOG
            || self.inodesize != (1 << self.inodelog)
            || self.blocklog - self.inodelog != self.inopblog
            || self.rextsize * self.blocksize > XfsSuperBlock::MAX_RTEXTSIZE
            || self.rextsize * self.blocksize < XfsSuperBlock::MIN_RTEXTSIZE
            || self.imax_pct > 100
            || self.dblocks == 0
            || self.dblocks.get() > self.max_dblocks()
            || self.dblocks.get() < self.min_dblocks()
        {
            return Err(XfsError::InvalidHeaderRanges);
        }

        if (self.versionnum.get() & 0x0f) == 5 {
            if (self.versionnum.get() & XfsSuperBlock::SB_VERSION_MOREBITSBIT) == 0 {
                return Err(XfsError::InvalidHeaderVersion);
            };

            if (self.features2.get() & XfsSuperBlock::SB_VERSION2_CRCBIT) == 0 {
                return Err(XfsError::InvalidHeaderFeatures);
            };

            #[cfg(feature = "std")]
            {
                use crc_fast::crc32_iscsi;

                crc_area[offset_of!(XfsSuperBlock, crc)..offset_of!(XfsSuperBlock, spino_align)]
                    .fill(0);

                let calc_sum = crc32_iscsi(crc_area);

                if self.crc.get() != calc_sum {
                    return Err(XfsError::HeaderChecksumInvalid);
                }
            }
        }
        return Ok(());
    }

    pub fn fssize(&self) -> u64 {
        let lsize = if self.logstart.get() != 0 {
            self.logblocks.get()
        } else {
            0
        };

        let avail_blocks = self.dblocks.get() - u64::from(lsize);
        let fssize = avail_blocks * u64::from(self.blocksize);

        return fssize;
    }
}

pub fn probe_xfs<IO: BlockIo>(
    reader: &mut Reader<IO>,
    flags: ProbeFlags,
    offset: u64,
    magic: Magic,
) -> Result<BlockInfo, Error<IO::Error>> {
    let buf: [u8; size_of::<XfsSuperBlock>()] = reader.read_exact_at(offset)?;
    let sb: &XfsSuperBlock = transmute_ref!(&buf);
    let mut crc_area = reader.read_vec_at(offset, usize::from(sb.sectsize))?;

    sb.verify(&mut crc_area)?;

    let label = if sb.fname[0] != 0 {
        if flags.contains(ProbeFlags::FailOnInvaildUTF) {
            Some(decode_utf8_from(&sb.fname).map_err(XfsError::Utf8Error)?)
        } else {
            Some(decode_utf8_lossy_from(&sb.fname))
        }
    } else {
        None
    };

    let mut info = BlockInfo::new();

    info.set(BlockTag::BlockType(BlockType::Xfs));
    info.set(BlockTag::Id(Id::Uuid(Uuid::from_bytes(sb.uuid))));
    if let Some(l) = label {
        info.set(BlockTag::Label(l));
    }
    info.set(BlockTag::Usage(Usage::Filesystem));
    info.set(BlockTag::Magic(magic.magic.to_vec()));
    info.set(BlockTag::MagicOffset(magic.b_offset));
    info.set(BlockTag::FsSize(sb.fssize()));
    info.set(BlockTag::FsLastBlock(sb.dblocks.get()));
    info.set(BlockTag::FsBlockSize(u64::from(sb.blocksize)));
    info.set(BlockTag::BlockSize(u64::from(sb.sectsize)));

    return Ok(info);
}
