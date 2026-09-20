use std::{
    io::{self, ErrorKind, stdout},
    path::PathBuf,
};

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use libblockid::{
    Probe,
    error::Error,
    filesystem::{FS_DETECT_ORDER, FsFilter, FsInfo},
    partition::{PT_DETECT_ORDER, PtFilter, PtInfo},
};
use serde::Serialize;
use serde_dotenv::to_writer as to_dotenv_writer;
use serde_json::to_writer_pretty as to_json_writer;
use shadow_rs::shadow;
use toml::to_string_pretty as to_toml_pretty;

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

        /// Set output format to list probed data
        #[arg(short = 'f', long = "format", value_enum)]
        format: Option<Format>,

        /// Set filter for what filesystems to probe for
        #[arg(long = "fs-filter", value_parser = ["apfs", "cramfs", "exfat", "jbd", "ext2", "ext3", "ext4", "luks1", "luks2", "luks_opal", "ntfs", "squashfs", "squashfs3", "vfat", "vxfs", "xfs"], num_args = 1.., value_delimiter = ',')]
        filesystem: Option<Vec<String>>,

        /// Set filter for what partition tables to probe for
        #[arg(long = "pt-filter", value_parser = ["aix", "mbr", "gpt"], num_args = 1.., value_delimiter = ',')]
        part_table: Option<Vec<String>>,
    },

    /// Display I/O topology of a device
    Topology {
        /// Block device path to probe (e.g. /dev/sda)
        #[arg(short = 'd', long = "device", value_name = "PATH")]
        device: PathBuf,

        /// Set output format to list topology infomation
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
    /// Output in TOML.
    Toml,
}

#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
struct Topology {
    device_size: u64,
    logical_sector_size: u64,
    physical_sector_size: u64,
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    minimum_io_size: u64,
    #[cfg(target_os = "linux")]
    optimal_io_size: u64,
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    alignment_offset: Option<u64>,
}

fn write_output<T: Serialize>(value: &T, format: Option<Format>) -> Result<(), Error<io::Error>> {
    match format.unwrap_or_default() {
        Format::Export => {
            to_dotenv_writer(stdout(), value).map_err(|_| Error::Io(ErrorKind::Other.into()))?
        }
        Format::Json => {
            to_json_writer(stdout(), value).map_err(|_| Error::Io(ErrorKind::Other.into()))?
        }
        Format::Toml => {
            let out = to_toml_pretty(value).map_err(|_| Error::Io(ErrorKind::Other.into()))?;
            print!("{out}")
        }
    }
    Ok(())
}

enum ProbeResult {
    PartTable(PtInfo),
    Filesystem(FsInfo),
}

impl Serialize for ProbeResult {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            ProbeResult::PartTable(p) => p.serialize(s),
            ProbeResult::Filesystem(f) => f.serialize(s),
        }
    }
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
        for (_, block) in FS_DETECT_ORDER {
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
                filesystem,
                part_table,
            } => {
                let mut probe = Probe::open(device, offset.unwrap_or_default())?;

                let pt_filter = match part_table {
                    Some(items) => {
                        let mut filter = PtFilter::empty();
                        for str in items {
                            filter.insert(
                                PtFilter::from_name(&str).expect("CLAP SHOULD CHECK INPUTS"),
                            );
                        }
                        filter
                    }
                    None => PtFilter::all(),
                };

                let fs_filter = match filesystem {
                    Some(items) => {
                        let mut filter = FsFilter::empty();
                        for str in items {
                            filter.insert(
                                FsFilter::from_name(&str).expect("CLAP SHOULD CHECK INPUTS"),
                            );
                        }
                        filter
                    }
                    None => FsFilter::all(),
                };

                let result = match probe.probe_part_table(pt_filter) {
                    Ok(info) => ProbeResult::PartTable(info),
                    Err(Error::Io(e)) => return Err(Error::Io(e)),
                    Err(_) => match probe.probe_filesystem(fs_filter) {
                        Ok(info) => ProbeResult::Filesystem(info),
                        Err(Error::Io(e)) => return Err(Error::Io(e)),
                        Err(_) => return Err(Error::ProbesExhausted),
                    },
                };

                write_output(&result, format)?;
            }
            Commands::Topology { device, format } => {
                let probe = Probe::open(device, 0)?;

                let topology = Topology {
                    device_size: probe.device_size()?,
                    logical_sector_size: probe.logical_sector_size()?,
                    physical_sector_size: probe.physical_sector_size()?,
                    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
                    minimum_io_size: probe.minimum_io_size()?,
                    #[cfg(target_os = "linux")]
                    optimal_io_size: probe.optimal_io_size()?,
                    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
                    alignment_offset: probe.alignment_offset()?.into(),
                };

                write_output(&topology, format)?;
            }
        }
    } else {
        Cli::command().print_help().unwrap();
    }
    Ok(())
}
