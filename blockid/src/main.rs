use std::{io::{Error as IoError, ErrorKind}, path::PathBuf};
use std::str::FromStr;
use libblockid::{BlockidProbe, ProbeFilter, ProbeFlags, BlockidError as LibblockidError,
    devno_to_path, ProbeResult};
use clap::{Arg, value_parser, ArgAction, Command, ValueEnum, builder::EnumValueParser};
use thiserror::Error;

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

#[derive(Debug, Error)]
pub enum BlockidError {
    #[error("IO error occured: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Libblockid Error: {0}")]
    LibblockidError(#[from] LibblockidError),
}

#[derive(Debug, Clone, ValueEnum)]
enum OutputType {
    Full,
    Device,
    Export,
    Json,
}

fn output_full(info: &[&BlockidProbe]) -> Result<(), BlockidError>{
    for probe in info {
        let device_path = devno_to_path(probe.disk_devno())?
            .to_string_lossy()
            .to_string();
        let results = probe.results()
            .ok_or(IoError::new(ErrorKind::InvalidData, "Will change this"))?;

        for result in results {
            match result {
                ProbeResult::Container(r) => {
                    println!("todo")
                },
                ProbeResult::Filesystem(r) => {
                    println!("{device_path} UUID=\"{}\"", r.fs_uuid.unwrap())
                },
                ProbeResult::PartTable(r) => {
                    println!("todo")
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), BlockidError> {
    //let code = rustix::ioctl::opcode::read::<u64>(b'd', 25);
    //println!("{code}");

    let matches = Command::new("blockid")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Lists block infomation")
        .arg(
            Arg::new("cache-file")
                .short('c')
                .long("cache-file")
                .help("Read from <file> instead of reading from the default")
                .value_name("file")
                .required(false)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("list-supported")
                .short('k')
                .long("list-supported")
                .help("List all known super blocks and exit")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("format")
                .help("Output format")
                .value_parser(EnumValueParser::<OutputType>::new())
                .default_value("full"),
        )
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .help("Turn debugging information on")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("probe")
                .short('p')
                .long("probe")
                .help("low-level superblocks probing (bypass cache)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("device")
                .help("Scan Specific device")
                .required(false)
                .index(1),
        )
        .get_matches();
    
    let cache = if let Some(cache_file) = matches.get_one::<PathBuf>("cache-file") {
        cache_file
    } else {
        &PathBuf::from_str(env!("CACHE_PATH")).expect("Unable to get CACHE_PATH to use as default")
    };

    if matches.get_flag("list-supported") {
        for item in BlockidProbe::list_supported_sb() {
            println!("{item}");   
        }
        return Ok(());
    }

    let output: Option<&OutputType> = matches.get_one("output");



    return Ok(());
}
