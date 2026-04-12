use rustix::{
    fd::AsFd,
    io,
    ioctl::{Getter, ioctl},
};


/// #define DKIOCGETBLOCKSIZE _IOR('d', 24, uint32_t)
const DKIOCGETBLOCKSIZE: u64 = 1074029592;

// /// #define DKIOCGETBLOCKCOUNT _IOR('d', 25, uint64_t)
// const DKIOCGETBLOCKCOUNT: u64 = 1074291737;

/// #define DKIOCGETPHYSICALBLOCKSIZE _IOR('d', 77, uint32_t)
const DKIOCGETPHYSICALBLOCKSIZE: u64 = 1074029645;

#[inline]
pub fn ioctl_dkiocgetblocksize<Fd: AsFd>(fd: Fd) -> io::Result<u32> {
    unsafe {
        let ctl = Getter::<{ DKIOCGETBLOCKSIZE }, u32>::new();
        ioctl(fd, ctl)
    }
}

// #[inline]
// pub fn ioctl_dkiocgetblockcount<Fd: AsFd>(fd: Fd) -> io::Result<u64> {
//     unsafe {
//         let ctl = Getter::<{ DKIOCGETBLOCKCOUNT }, u64>::new();
//         ioctl(fd, ctl)
//     }
// }

#[inline]
pub fn ioctl_dkiocgetphysicalblocksize<Fd: AsFd>(fd: Fd) -> io::Result<u32> {
    unsafe {
        let ctl = Getter::<{ DKIOCGETPHYSICALBLOCKSIZE }, u32>::new();
        ioctl(fd, ctl)
    }
}
