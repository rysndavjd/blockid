pub mod mbr;
pub mod ext4;

//pub fn test(device: &str) -> Result<(), Box<dyn std::error::Error>> {
//    let mut superblock = File::open(device)?;
//    let mut buffer = [0; 512];
//    superblock.read_exact(&mut buffer)?;
//
//    println!("{:X?}", buffer);
//    return Ok(());
//
//}
