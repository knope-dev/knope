#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::cargo)]
#![allow(clippy::multiple_crate_versions)] // Let cargo-deny handle this
#![forbid(unsafe_code)]

use clap::Parser;
use miette::Result;
use std::env::var;

use knope::{run, Cli};

fn main() -> Result<()> {
    if var("RUST_LOG").is_ok() {
        env_logger::init();
    }
    run(Cli::parse())
}
