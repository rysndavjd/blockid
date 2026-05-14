use std::{
    fs::File,
    os::fd::{AsFd, AsRawFd},
};

use clap::{ArgAction, Parser};
use libblockid::{Probe, ProbeFlags, fd_to_path, filesystem::BlockFilter, partition::PTFilter};
use rustix::fd::{FromRawFd, OwnedFd};
use shadow_rs::{Format, shadow};

shadow!(build);

#[derive(Parser)]
#[command(version)]
#[command(about, long_about)]
struct Cli {
    /// Print long version from build time
    #[arg(long = "long-version", action = ArgAction::SetTrue)]
    version_long: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.version_long {
        build::print_build_in();
    }

    let file = File::open("/dev/sdb").unwrap();

    let p = fd_to_path(file.as_fd()).unwrap();

    println!("{:?}", p);

    let mut t = Probe::from_file(file, ProbeFlags::empty(), 0).unwrap();

    let info = t.search_for_part_table(libblockid::partition::PTType::Gpt);

    println!("{:?}", info);
}
