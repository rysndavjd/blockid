use clap::{ArgAction, Parser};
use libblockid_sys::{BlockFilter, Probe};
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
}
