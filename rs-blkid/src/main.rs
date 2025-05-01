use rs_libblkid::ext4::*;
use uuid::Uuid;
use std::str::from_utf8;

fn ext4_into() -> Result<(), Box<dyn std::error::Error>> {
    let device = read_ext4("/dev/sdb1")?;
    //let hex_string: String = device.s_volume_name
    //                .iter()
    //                .map(|&b| format!("{:02x}", b))
    //                .collect();
    println!("{}", from_utf8(&device.s_volume_name)?);

    return Ok(());
}

fn main() {
    match ext4_into() {
        Ok(t) => t,
        Err(e) => eprintln!("{}", e),
    };
}
