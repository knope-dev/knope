#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::cargo)]
#![forbid(unsafe_code)]

use color_eyre::Result;

use dobby::{command, run};

fn main() -> Result<()> {
    color_eyre::install().expect("Could not set up error handling with color_eyre");

    run(&command().get_matches())
}
