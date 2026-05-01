use std::{
    fs::File,
    os::fd::{AsFd, AsRawFd},
};

use clap::{ArgAction, Parser};
use libblockid::{BlockFilter, Probe};
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

    let file = File::open("/dev/nvme0n1p3").unwrap();

    let mut t = Probe::new(unsafe { OwnedFd::from_raw_fd(file.as_raw_fd()) }).unwrap();

    let info = t.probe_block(0, BlockFilter::empty()).unwrap();

    println!("{:?}", info);
}
