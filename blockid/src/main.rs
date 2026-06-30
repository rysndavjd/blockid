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
    command: Commands,

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

// fn print_probe_pt(format: Format, info: PartTableInfo) {
//     match format {
//         Format::Export => {
//             if let Some(pt_type) = info.pt_type() {
//                 println!("PT_TYPE=\"{}\"", pt_type);

//                 if let Some(id) = info.id() {
//                     match pt_type {
//                         PTType::Aix => (),
//                         PTType::Gpt => {
//                             println!(
//                                 "PT_ID=\"{}\"",
//                                 id.as_uuid().expect("should be a uuid with GPT")
//                             )
//                         }
//                         PTType::Mbr => {
//                             println!(
//                                 "PT_ID=\"{}\"",
//                                 id.as_mbr().expect("should be a u32 with MBR")
//                             )
//                         }
//                         _ => (),
//                     }
//                 }

//                 if let Some(pt_size) = info.pt_size() {
//                     println!("PT_SIZE=\"{}\"", pt_size);
//                 }

//                 if let Some(magic) = info.magic() {
//                     println!("PT_MAGIC=\"{:x?}\"", magic);
//                 }

//                 if let Some(magic_offset) = info.magic_offset() {
//                     println!("PT_MAGIC_OFFSET=\"{:?}\"", magic_offset);
//                 }

//                 if let Some(partitions) = info.partitions() {
//                     for partition in partitions {
//                         println!("PART{}_START=\"{}\"", partition.part_no, partition.start);
//                         println!("PART{}_END=\"{}\"", partition.part_no, partition.end);
//                         match pt_type {
//                             PTType::Gpt => {
//                                 println!(
//                                     "PART{}_ID=\"{}\"",
//                                     partition.part_no,
//                                     partition
//                                         .partition_id
//                                         .as_uuid()
//                                         .expect("should be a uuid with GPT")
//                                 );
//                             }
//                             _ => todo!(),
//                         }

//                         if let Some(name) = &partition.partition_name {
//                             println!("PART{}_NAME=\"{}\"", partition.part_no, name);
//                         }
//                     }
//                 }
//             }
//         }
//         Format::Json => {
//             todo!()
//         }
//     }
// }

#[allow(non_snake_case)]
#[derive(Serialize)]
struct Topology {
    DEVICE_SIZE: u64,
    LOGICAL_SECTOR_SIZE: u64,
    PHYSICAL_SECTOR_SIZE: u64,
    MINIMUM_IO_SIZE: u64,
    OPTIMAL_IO_SIZE: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    ALIGNMENT_OFFSET: Option<u64>,
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

    match cli.command {
        Commands::Probe {
            device,
            offset,
            format,
            blocks,
        } => {
            let mut probe = Probe::open(device, ProbeFlags::empty(), offset.unwrap_or_default())?;

            match probe.probe_part_table(PTFilter::empty()) {
                Ok(info) => {
                    match format.unwrap_or_default() {
                        Format::Export => {
                            for tag in info.inner() {
                                match tag {
                                    PartTableTag::PartTableType(t) => {
                                        to_dotenv_writer(stdout(), t).unwrap()
                                    }
                                    PartTableTag::PartTableId(t) => {
                                        to_dotenv_writer(stdout(), t).unwrap()
                                    }
                                    PartTableTag::PartTableSize(t) => {
                                        to_dotenv_writer(stdout(), t).unwrap()
                                    }
                                    // PartTableTag::Magic(t) => {
                                    //     to_dotenv_writer(stdout(), t).unwrap()
                                    // }
                                    PartTableTag::MagicOffset(t) => {
                                        to_dotenv_writer(stdout(), t).unwrap()
                                    }
                                    // PartTableTag::Partitions(t) => {
                                    //     to_dotenv_writer(stdout(), t).unwrap()
                                    // }
                                    _ => (),
                                }
                                println!()
                            }
                        }
                        Format::Json => {
                            todo!();
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
                    println!("{:?}", info);
                    todo!();
                    return Ok(());
                }
                Err(e) => {
                    if let Error::Io(_) = e {
                        return Err(e);
                    }
                }
            }

            Err(Error::ProbesExhausted)
        }
        Commands::Topology { device, format } => {
            let probe = Probe::open(device, ProbeFlags::empty(), 0)?;

            let topology = Topology {
                DEVICE_SIZE: probe.device_size()?,
                LOGICAL_SECTOR_SIZE: probe.logical_sector_size()?,
                PHYSICAL_SECTOR_SIZE: probe.physical_sector_size()?,
                MINIMUM_IO_SIZE: probe.minimum_io_size()?,
                OPTIMAL_IO_SIZE: probe.optimal_io_size()?,
                ALIGNMENT_OFFSET: probe.alignment_offset()?.into(),
            };

            match format.unwrap_or_default() {
                Format::Export => {
                    to_dotenv_writer(stdout(), &topology).expect("ahh, making dotenv failed.");
                    println!();
                }
                Format::Json => {
                    to_json_writer(stdout(), &topology).expect("ahh, making JSON pretty failed.");
                    println!();
                }
            }

            Ok(())
        }
    }
}
