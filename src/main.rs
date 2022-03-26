#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::cargo)]
#![allow(clippy::multiple_crate_versions)] // Let cargo-deny handle this
#![forbid(unsafe_code)]

use clap::Parser;
use miette::Result;

use dobby::{run, Cli};

fn main() -> Result<()> {
    run(Cli::parse())
}
