use std::fs::File;

use libblockid_core::{Filter, LowProbe};

fn main() {
    let file = File::open("/dev/sdb1").unwrap();

    let mut probe = LowProbe::new(file, 0, Filter::empty());

    let ext4 = probe.probe().unwrap();

    println!("{:?}", ext4);
}
