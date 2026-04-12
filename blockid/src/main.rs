use alloc::format;
use core::{panic::PanicInfo, str::FromStr};
use libblockid_sys::{BlockFilter, Probe};
use rustix::{
    fd::OwnedFd,
    fs::{Mode, OFlags, open},
    ioctl::opcode::*,
};
extern crate alloc;

fn main() {
    let file = open("/dev/nvme0n1p3", OFlags::RDONLY, Mode::empty()).unwrap();
    // let file = File::open("/dev/nvme0n1p3").unwrap();
    let mut p = Probe::new(file).unwrap();

    let t = p.probe_info(0, BlockFilter::empty()).unwrap();

    let t = format!("{:?}\n", t);

    rustix::io::write(rustix::stdio::stdout(), t.as_bytes()).unwrap();
}
