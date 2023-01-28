#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::cargo)]
#![allow(clippy::multiple_crate_versions)] // Let cargo-deny handle this
#![forbid(unsafe_code)]

use std::env::var;

use clap::Parser;
use knope::{run, Cli};
use miette::Result;

fn main() -> Result<()> {
    if var("RUST_LOG").is_ok() {
        env_logger::init();
    }
    run(Cli::parse())
}
