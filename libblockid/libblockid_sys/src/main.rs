use std::fs::File;

use libblockid_core::{Filter, LowProbe};

fn main() {
    let file = File::open("/dev/nvme0n1p5").unwrap();

    let mut probe = LowProbe::new(file, 0, Filter::empty());

    let ext4 = probe.probe().unwrap();

    println!("{:?}", ext4);
}
