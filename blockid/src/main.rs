use std::{
    io::{self, stdout},
    path::PathBuf,
};

use clap::{Parser, Subcommand, ValueEnum};
use libblockid::{
    AlignmentOffset, Probe, ProbeFlags,
    error::Error,
    filesystem::{BLOCK_DETECT_ORDER, BlockFilter, BlockInfo, BlockType},
    partition::{
        PT_DETECT_ORDER, PTFilter, PartTableInfo, PartTableTag, PartTableType, Partition,
        PartitionType,
    },
};
use serde::Serialize;
use serde_dotenv::to_writer as to_dotenv_writer;
use serde_json::to_writer_pretty as to_json_writer;
use shadow_rs::shadow;

shadow!(build);

#[derive(Parser)]
#[command(version = build::PKG_VERSION)]
#[command(long_version = build::CLAP_LONG_VERSION)]
#[command(about, long_about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// List all known supported superblocks
    #[arg(short = 'k', long = "list-superblocks")]
    avail_sb: bool,

    /// Print version of crate and its dependencies
    #[arg(long = "shadow-version")]
    shadow_version: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Probe a device for filesystem and partition superblock information
    Probe {
        /// Block device path to probe (e.g. /dev/sda)
        #[arg(short = 'd', long = "device", value_name = "PATH")]
        device: PathBuf,

        /// Set the start offset in bytes to begin probing at
        #[arg(short = 'o', long = "offset", value_name = "BYTES")]
        offset: Option<u64>,

        /// Set output format to list probed data.
        #[arg(short = 'f', long = "format", value_enum)]
        format: Option<Format>,

        /// Set filter for what superblock type to parse for.
        #[arg(short = 't', long = "type-filter", value_enum)]
        blocks: Option<Vec<String>>,
    },

    /// Display I/O topology of a device
    Topology {
        /// Block device path to probe (e.g. /dev/sda)
        #[arg(short = 'd', long = "device", value_name = "PATH")]
        device: PathBuf,

        /// Set output format to list topology infomation.
        #[arg(short = 'f', long = "format", value_enum)]
        format: Option<Format>,
    },
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, ValueEnum, Default)]
enum Format {
    /// Output in dotenv.
    #[default]
    Export,
    /// Output in JSON.
    Json,
}

#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
struct Topology {
    device_size: u64,
    logical_sector_size: u64,
    physical_sector_size: u64,
    minimum_io_size: u64,
    optimal_io_size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    alignment_offset: Option<u64>,
}

fn main() {
    if let Err(e) = _main() {
        eprintln!("{}", e)
    }
}

fn _main() -> Result<(), Error<io::Error>> {
    let cli = Cli::parse();

    if cli.shadow_version {
        build::print_build_in();
        return Ok(());
    }

    if cli.avail_sb {
        for (_, pt) in PT_DETECT_ORDER {
            println!("{}", pt)
        }
        for (_, block) in BLOCK_DETECT_ORDER {
            println!("{}", block)
        }
        return Ok(());
    }

    if let Some(command) = cli.command {
        match command {
            Commands::Probe {
                device,
                offset,
                format,
                blocks,
            } => {
                let mut probe =
                    Probe::open(device, ProbeFlags::empty(), offset.unwrap_or_default())?;

                match probe.probe_part_table(PTFilter::empty()) {
                    Ok(info) => {
                        match format.unwrap_or_default() {
                            Format::Export => {
                                to_dotenv_writer(stdout(), &info).unwrap();
                            }
                            Format::Json => {
                                to_json_writer(stdout(), &info).unwrap();
                            }
                        }

                        return Ok(());
                    }
                    Err(e) => {
                        if let Error::Io(_) = e {
                            return Err(e);
                        }
                    }
                }

                match probe.probe_block(BlockFilter::empty()) {
                    Ok(info) => {
                        match format.unwrap_or_default() {
                            Format::Export => {
                                to_dotenv_writer(stdout(), &info).unwrap();
                            }
                            Format::Json => {
                                to_json_writer(stdout(), &info).unwrap();
                            }
                        }

                        return Ok(());
                    }
                    Err(e) => {
                        if let Error::Io(_) = e {
                            return Err(e);
                        }
                    }
                }

                return Err(Error::ProbesExhausted);
            }
            Commands::Topology { device, format } => {
                let probe = Probe::open(device, ProbeFlags::empty(), 0)?;

                let topology = Topology {
                    device_size: probe.device_size()?,
                    logical_sector_size: probe.logical_sector_size()?,
                    physical_sector_size: probe.physical_sector_size()?,
                    minimum_io_size: probe.minimum_io_size()?,
                    optimal_io_size: probe.optimal_io_size()?,
                    alignment_offset: probe.alignment_offset()?.into(),
                };

                match format.unwrap_or_default() {
                    Format::Export => {
                        to_dotenv_writer(stdout(), &topology).expect("ahh, making dotenv failed.");
                        println!();
                    }
                    Format::Json => {
                        to_json_writer(stdout(), &topology)
                            .expect("ahh, making JSON pretty failed.");
                        println!();
                    }
                }
            }
        }
    }
    Ok(())
}
