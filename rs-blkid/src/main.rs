use rs_libblkid::*;
use rs_libblkid::Ext4SuperBlock;

fn ext4_into() -> Result<(), Box<dyn std::error::Error>> {
    let device = read_ext4("/dev/sdb1")?;
    println!("{:?}", device.s_inodes_count);
    return Ok(());
}

fn main() {
    ext4_into();
}
