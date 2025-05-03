use rs_libblkid::fat::*;
use uuid::Uuid;

fn test() -> Result<(), Box<dyn std::error::Error>> {
    
    let device = read_raw_fat32_ext_bs("/dev/sdb3")?;
    let id = device.volume_id;

    println!("{:?}", format!("{:X}", id));
    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{}", e),
    };
}
