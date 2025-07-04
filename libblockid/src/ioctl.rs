use std::os::fd::AsFd;

use linux_raw_sys::ioctl::BLKGETSIZE64;
use rustix::{io, ioctl::{ioctl, Getter}};

#[inline]
pub fn ioctl_blkgetsize64<Fd: AsFd>(fd: Fd) -> io::Result<u64> {
    unsafe {
        let ctl = Getter::<{ BLKGETSIZE64 }, u64>::new();
        ioctl(fd, ctl)
    }
}

