use rustix::{
    fd::AsFd,
    io,
    ioctl::{Getter, ioctl, opcode::read},
};

/// [#define DKIOCGETBLOCKSIZE _IOR('d', 24, uint32_t)](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/sys/disk.h#L185)
/// 1074029592
const DKIOCGETBLOCKSIZE: u64 = read::<u32>(b'd', 24);

/// [#define DKIOCGETBLOCKCOUNT _IOR('d', 25, uint64_t)](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/sys/disk.h#L186)
/// 1074291737
const DKIOCGETBLOCKCOUNT: u64 = read::<u64>(b'd', 25);

/// [#define DKIOCGETPHYSICALBLOCKSIZE _IOR('d', 77, uint32_t)](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/sys/disk.h#L212)
/// 1074029645
const DKIOCGETPHYSICALBLOCKSIZE: u64 = read::<u32>(b'd', 77);

#[inline]
pub fn ioctl_dkiocgetblocksize<Fd: AsFd>(fd: Fd) -> io::Result<u32> {
    unsafe {
        let ctl = Getter::<{ DKIOCGETBLOCKSIZE }, u32>::new();
        ioctl(fd, ctl)
    }
}

#[inline]
pub fn ioctl_dkiocgetblockcount<Fd: AsFd>(fd: Fd) -> io::Result<u64> {
    unsafe {
        let ctl = Getter::<{ DKIOCGETBLOCKCOUNT }, u64>::new();
        ioctl(fd, ctl)
    }
}

#[inline]
pub fn ioctl_dkiocgetphysicalblocksize<Fd: AsFd>(fd: Fd) -> io::Result<u32> {
    unsafe {
        let ctl = Getter::<{ DKIOCGETPHYSICALBLOCKSIZE }, u32>::new();
        ioctl(fd, ctl)
    }
}
