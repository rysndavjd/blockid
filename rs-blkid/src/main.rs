use rs_libblkid::*;

fn main() {
    match read_ext4("/dev/sdb1") {
        Ok(t) => t,
        Err(e) => println!("{}", e),
    }  
}
