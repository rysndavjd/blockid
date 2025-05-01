use rs_libblkid::*;
use uuid::Uuid;

fn ext4_into() -> Result<(), Box<dyn std::error::Error>> {
    let device = read_ext4("/dev/sdb1")?;
    //println!("Raw values {:?}", device.s_uuid);
    println!("UUID string: {:?}", device.s_state);
    return Ok(());
}

fn main() {
    match ext4_into() {
        Ok(t) => t,
        Err(e) => eprintln!("{}", e),
    };
}
