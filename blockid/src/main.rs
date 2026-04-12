use std::fs::File;

use rustix::ioctl::opcode::*;
use libblockid_sys::{BlockFilter, Probe};


fn main() {
    let file = File::open("/dev/disk0s4").unwrap();
    let mut p = Probe::new(file).unwrap();

    let t = p.probe_info(0, BlockFilter::empty()).unwrap();

    println!("{:?}", t);
}
