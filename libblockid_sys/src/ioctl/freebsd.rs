use rustix::{
    fd::AsFd,
    io,
    ioctl::{Getter, ioctl},
};

/*
 * off_t = 8 bytes
 * #define DIOCGMEDIASIZE _IOR('d', 129, off_t)
 */

const DIOCGMEDIASIZE: u64 = 1074291841;

/*
 * u_int = 4 bytes
 * #define	DIOCGSECTORSIZE	_IOR('d', 128, u_int)
 */

const DIOCGSECTORSIZE: u64 = 1074029696;

#[inline]
pub fn ioctl_diocgsectorsize<Fd: AsFd>(fd: Fd) -> io::Result<u32> {
    unsafe {
        let ctl = Getter::<{ DIOCGSECTORSIZE }, u32>::new();
        ioctl(fd, ctl)
    }
}

#[inline]
pub fn ioctl_diocgmediasize<Fd: AsFd>(fd: Fd) -> io::Result<u64> {
    unsafe {
        let ctl = Getter::<{ DIOCGMEDIASIZE }, u64>::new();
        ioctl(fd, ctl)
    }
}
