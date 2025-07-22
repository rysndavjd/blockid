use bitflags::bitflags;
use rustix::{io, ioctl::{ioctl, Getter}, fd::AsFd};

#[cfg(target_os = "linux")]
pub const BLKGETSIZE64: u32 = 2148012658;

#[cfg(target_os = "linux")]
const IOC_OPAL_GET_STATUS: u32 = 2148036844;

/* 
 * off_t = 8 bytes
 * #define DIOCGMEDIASIZE _IOR('d', 129, off_t) 
 */

#[cfg(target_os = "freebsd")]
const DIOCGMEDIASIZE: u64 = 2148033665;

/* 
 * u_int = 4 bytes
 * #define	DIOCGSECTORSIZE	_IOR('d', 128, u_int)
 */

#[cfg(target_os = "freebsd")]
const DIOCGSECTORSIZE: u64 = 2147771520;

/* 
 * uint32_t = 4 bytes
 * #define DKIOCGETBLOCKSIZE _IOR('d', 24, uint32_t)
 */

#[cfg(target_os = "macos")]
const DKIOCGETBLOCKSIZE: u32 = 2147771416;

/* 
 * uint64_t = 8 bytes
 * #define DKIOCGETBLOCKCOUNT _IOR('d', 25, uint64_t)
 */

#[cfg(target_os = "macos")]
const DKIOCGETBLOCKCOUNT: u32 = 2148033561;

#[cfg(target_os = "linux")]
#[inline]
pub fn ioctl_blkgetsize64<Fd: AsFd>(fd: Fd) -> io::Result<u64> {
    unsafe {
        let ctl = Getter::<{ BLKGETSIZE64 }, u64>::new();
        ioctl(fd, ctl)
    }
}

#[cfg(target_os = "freebsd")]
#[inline]
pub fn ioctl_diocgsectorsize<Fd: AsFd>(fd: Fd) -> io::Result<u32> {
    unsafe {
        let ctl = Getter::<{ DIOCGSECTORSIZE }, u32>::new();
        ioctl(fd, ctl)
    }
}

#[cfg(target_os = "freebsd")]
#[inline]
pub fn ioctl_diocgmediasize<Fd: AsFd>(fd: Fd) -> io::Result<u64> {
    unsafe {
        let ctl = Getter::<{ DIOCGMEDIASIZE }, u64>::new();
        ioctl(fd, ctl)
    }
}

#[cfg(target_os = "macos")]
#[inline]
pub fn ioctl_dkiocgetblocksize<Fd: AsFd>(fd: Fd) -> io::Result<u32> {
    unsafe {
        let ctl = Getter::<{ DKIOCGETBLOCKSIZE }, u32>::new();
        ioctl(fd, ctl)
    }
}

#[cfg(target_os = "macos")]
#[inline]
pub fn ioctl_dkiocgetblockcount<Fd: AsFd>(fd: Fd) -> io::Result<u64> {
    unsafe {
        let ctl = Getter::<{ DKIOCGETBLOCKCOUNT }, u64>::new();
        ioctl(fd, ctl)
    }
}

#[cfg(target_os = "linux")]
bitflags!{
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct OpalStatusFlags: u32 {
        const OPAL_FL_SUPPORTED         = 0x00000001;
        const OPAL_FL_LOCKING_SUPPORTED = 0x00000002; 
        const OPAL_FL_LOCKING_ENABLED   = 0x00000004;
        const OPAL_FL_LOCKED            = 0x00000008;
        const OPAL_FL_MBR_ENABLED       = 0x00000010;
        const OPAL_FL_MBR_DONE          = 0x00000020;
        const OPAL_FL_SUM_SUPPORTED     = 0x00000040;
    }
}

#[cfg(target_os = "linux")]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OpalStatus {
    pub flags: OpalStatusFlags,
    pub reserved: u32,
}

#[cfg(target_os = "linux")]
#[inline]
pub fn ioctl_ioc_opal_get_status<Fd: AsFd>(fd: Fd) -> io::Result<OpalStatus> {
    unsafe {
        let ctl = Getter::<{ IOC_OPAL_GET_STATUS }, OpalStatus>::new();
        ioctl(fd, ctl)
    }
}

#[inline]
pub fn logical_block_size<Fd: AsFd>(fd: Fd) -> io::Result<u32> {
    #[cfg(target_os = "linux")]
    return rustix::fs::ioctl_blksszget(fd);
    #[cfg(target_os = "freebsd")]
    return ioctl_diocgsectorsize(fd);
    #[cfg(target_os = "macos")]
    return ioctl_dkiocgetblocksize(fd);
}

#[inline]
pub fn device_size_bytes<Fd: AsFd>(fd: Fd) -> io::Result<u64> {
    #[cfg(target_os = "linux")]
    return ioctl_blkgetsize64(fd);
    #[cfg(target_os = "freebsd")]
    return ioctl_diocgmediasize(fd);
    #[cfg(target_os = "macos")]
    return Ok(ioctl_dkiocgetblocksize(fd)? * ioctl_dkiocgetblockcount(fd)?);
}