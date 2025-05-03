use rs_libblkid::fat::*;
use uuid::Uuid;

fn test() -> Result<(), Box<dyn std::error::Error>> {

    let device = testss("/dev/sdb4")?;
    //let id = device.volume_label;

    //println!("{:?}", format!("{:X?}", id));
    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{}", e),
    };
}
