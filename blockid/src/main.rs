use libblockid::{BlockidProbe, ProbeFilter, ProbeFlags};

fn test() -> Result<(), Box<dyn std::error::Error>> {
    //let file = File::open("/dev/sdb")?;

    let mut result = BlockidProbe::probe_from_filename("/dev/sdb2", ProbeFlags::empty(), ProbeFilter::empty(), 0)
        .unwrap();
    
    result.probe_values()?;
    //match probe_gpt_pt(&mut result, BlockidMagic::EMPTY_MAGIC) {
    //    Ok(_) => println!("Ok"),
    //    Err(e) => println!("{}", e),
    //}

    println!("{:?}", result);
    
    return Ok(());
}

fn main() {
    match test() {
        Ok(t) => t,
        Err(e) => eprintln!("{:?}", e),
    };
}
