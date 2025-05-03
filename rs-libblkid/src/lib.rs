// Partition Tables
pub mod mbr;
pub mod gpt;

// Filesystems
pub mod ext4;
pub mod fat;

/*
ideas

make a probe function with a struct
that has basic info for filesystems and partitions eg like
struct filesystems
    file system uuid
    partition uuid 
    label
    filesystem type
    filesystem version 
    filesystem magic signature
    size of filesystem in bytes
    
struct partitions
    partition type
    partition table uuid/id
    partition name
    partition uuid
    partition number
    partition offset
    partition size
    disk maj:min
*/



//pub fn test(device: &str) -> Result<(), Box<dyn std::error::Error>> {
//    let mut superblock = File::open(device)?;
//    let mut buffer = [0; 512];
//    superblock.read_exact(&mut buffer)?;
//
//    println!("{:X?}", buffer);
//    return Ok(());
//
//}
