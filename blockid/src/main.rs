use std::{io::{Error as IoError, ErrorKind}, path::PathBuf};
use std::str::FromStr;
use libblockid::{BlockidProbe, ProbeFilter, ProbeFlags, BlockidError as LibblockidError,
    devno_to_path, ProbeResult};
use clap::{Arg, value_parser, ArgAction, Command, ValueEnum, builder::EnumValueParser,
    parser::ValuesRef};
use thiserror::Error;
use bitflags::bitflags;

const CACHE_PATH: &'static str = env!("CACHE_PATH");

#[derive(Debug, Error)]
pub enum BlockidError {
    #[error("IO error occured: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Invalid tag input: {0}")]
    TagError(String),
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

fn print_tags(probe: BlockidProbe, mut _tags: ValuesRef<OutputTags>, _format: OutputType) -> Result<(), BlockidError> {
    let mut tags = _tags.peekable();

    if tags.peek().is_none() {
        return Err(BlockidError::TagError(String::from("Please enter at least one tag to print")));
    }
    
    let results = probe
        .results()
        .ok_or(BlockidError::TagError(String::from("No value to print")))?;

    for result in results {
        let mut matched_tags: Vec<String> = Vec::new();

        match result {
            ProbeResult::Container(t) => {
                for tag in tags {
                    match tag {
                        &OutputTags::Device => matched_tags.push(devno_to_path(probe.devno())?.display().to_string()),
                        &OutputTags::Type => matched_tags.push(t.cont_type.unwrap().to_string()),
                        &OutputTags::Label => matched_tags.push(t.label.unwrap()),
                        &OutputTags::PartLabel => 
                        &OutputTags::Uuid => 
                        &OutputTags::PartUuid => 
                        &OutputTags::BlockSize => 
                        &OutputTags::Creator => 
                    }
                }
            }
        }
    }

    return Ok(());
}

fn main() -> Result<(), BlockidError> {
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
            Arg::new("match-tag")
                .short('s')
                .long("match-tag")
                .help("show specified tag(s)")
                .value_delimiter(',')
                .value_parser(EnumValueParser::<OutputTags>::new())
                .action(clap::ArgAction::Append)
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
        &PathBuf::from(CACHE_PATH)
    };

    if matches.get_flag("list-supported") {
        for item in BlockidProbe::list_supported_sb() {
            println!("{item}");   
        }
        return Ok(());
    }

    let mut tags: ValuesRef<OutputTags> = matches
        .get_many::<OutputTags>("match-tag")
        .ok_or(BlockidError::ClapError("Unable to convert tag to enum"))?;


    return Ok(());
}
