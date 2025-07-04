use std::os::fd::AsFd;

use bitflags::bitflags;
use linux_raw_sys::ioctl::BLKGETSIZE64;
use rustix::{io, ioctl::{ioctl, Getter}};

const IOC_OPAL_GET_STATUS: u32 = 2148036844;

#[inline]
pub fn ioctl_blkgetsize64<Fd: AsFd>(fd: Fd) -> io::Result<u64> {
    unsafe {
        let ctl = Getter::<{ BLKGETSIZE64 }, u64>::new();
        ioctl(fd, ctl)
    }
}

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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OpalStatus {
    pub flags: OpalStatusFlags,
    pub reserved: u32,
}

#[inline]
pub fn ioctl_ioc_opal_get_status<Fd: AsFd>(fd: Fd) -> io::Result<OpalStatus> {
    unsafe {
        let ctl = Getter::<{ IOC_OPAL_GET_STATUS }, OpalStatus>::new();
        ioctl(fd, ctl)
    }
}