use rustix::{
    fd::AsFd,
    io,
    ioctl::{
        Getter, ioctl,
        opcode::{none, read},
    },
};

pub const BLKGETSIZE64: u32 = read::<u64>(0x12, 114);
pub const BLKIOMIN: u32 = none(0x12, 120);
pub const BLKIOOPT: u32 = none(0x12, 121);
pub const BLKALIGNOFF: u32 = none(0x12, 122);

#[inline]
pub fn ioctl_blkgetsize64<Fd: AsFd>(fd: Fd) -> io::Result<u64> {
    unsafe {
        let ctl = Getter::<{ BLKGETSIZE64 }, u64>::new();
        ioctl(fd, ctl)
    }
}

#[inline]
pub fn ioctl_blkiomin<Fd: AsFd>(fd: Fd) -> io::Result<u32> {
    unsafe {
        let ctl = Getter::<{ BLKIOMIN }, u32>::new();
        ioctl(fd, ctl)
    }
}

#[inline]
pub fn ioctl_blkioopt<Fd: AsFd>(fd: Fd) -> io::Result<u32> {
    unsafe {
        let ctl = Getter::<{ BLKIOOPT }, u32>::new();
        ioctl(fd, ctl)
    }
}

#[inline]
pub fn ioctl_blkalignoff<Fd: AsFd>(fd: Fd) -> io::Result<i32> {
    unsafe {
        let ctl = Getter::<{ BLKALIGNOFF }, i32>::new();
        ioctl(fd, ctl)
    }
}
