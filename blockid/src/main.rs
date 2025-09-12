use bitflags::bitflags;
use clap::{
    Arg, ArgAction, Command, ValueEnum, builder::EnumValueParser, parser::ValuesRef, value_parser,
};
use libblockid::{BlockidError as LibblockidError, Probe, ProbeBuilder, devno_to_path};
use rustix::{fs::makedev, ioctl::opcode::read};
use simple_logger::init;
use std::{
    fs::File,
    io::{Error as IoError, ErrorKind},
    os::fd::AsFd,
    path::{Path, PathBuf},
};
use thiserror::Error;

const CACHE_PATH: &str = env!("CACHE_PATH");

#[derive(Debug, Error)]
pub enum BlockidError {
    #[error("IO error occured: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Invalid tag input: {0}")]
    TagError(&'static str),
    #[error("Clap error: {0}")]
    ClapError(&'static str),
    #[error("Libblockid Error: {0}")]
    LibblockidError(#[from] LibblockidError),
}

#[derive(Debug, Clone, ValueEnum)]
enum OutputType {
    Export,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputTags {
    Device,
    Type,
    Label,
    PartLabel,
    Uuid,
    PartUuid,
    BlockSize,
    Creator,
}

fn main() -> Result<(), BlockidError> {
    init().unwrap();

    let mut p = ProbeBuilder::new().path("/dev/sdb1").build().unwrap();
    p.enable_buffering_with_capacity(16834).unwrap();
    p.probe_values().unwrap();

    println!("{p:?}");

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
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("list-supported")
                .short('k')
                .long("list-supported")
                .help("List all known super blocks and exit")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("match-tag")
                .short('s')
                .long("match-tag")
                .help("show specified tag(s)")
                .value_delimiter(',')
                .value_parser(EnumValueParser::<OutputTags>::new())
                .default_missing_value("device,label,uuid,blocksize,type,partlabel,partuuid")
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("format")
                .help("Output format")
                .value_parser(EnumValueParser::<OutputType>::new()),
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

    let cache = if let Some(cache_file) = matches.get_one::<String>("cache-file") {
        PathBuf::from(cache_file)
    } else {
        PathBuf::from(CACHE_PATH)
    };

    if matches.get_flag("list-supported") {
        for item in Probe::supported_string() {
            println!("{item}");
        }
        return Ok(());
    }

    let tags: Vec<OutputTags> = match matches.get_many::<OutputTags>("match-tag") {
        Some(r) => r.into_iter().copied().collect(),
        None => {
            vec![
                OutputTags::Device,
                OutputTags::Label,
                OutputTags::Uuid,
                OutputTags::BlockSize,
                OutputTags::Type,
                OutputTags::PartLabel,
                OutputTags::PartUuid,
            ]
        }
    };

    return Ok(());
}
