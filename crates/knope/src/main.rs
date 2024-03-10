use std::env::var;

use knope::run;
use miette::Result;

fn main() -> Result<()> {
    if var("RUST_LOG").is_ok() {
        env_logger::init();
    }
    run()
}
