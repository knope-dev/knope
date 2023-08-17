//! Proxies to FS utils that _either_ actually write to files or print to stdout (for dry runs).

use std::{
    fmt::Display,
    io,
    io::Write,
    path::{Path, PathBuf},
};

use log::trace;
use miette::Diagnostic;
use thiserror::Error;

/// Writes to a file if this is not a dry run, or prints just the diff to stdout if it is.
pub(crate) fn write<C: AsRef<[u8]> + Display>(
    dry_run: &mut Option<Box<dyn Write>>,
    diff: &str,
    path: &Path,
    contents: C,
) -> Result<(), Error> {
    if let Some(stdout) = dry_run {
        writeln!(stdout, "Would write {} to {}", diff, path.display()).map_err(Error::Stdout)
    } else {
        trace!("Writing {} to {}", contents, path.display());
        std::fs::write(path, contents).map_err(|source| Error::File {
            path: path.into(),
            source,
        })
    }
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error("Error writing to {path}: {source}")]
    File {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Error writing to stdout: {0}")]
    Stdout(#[from] io::Error),
}
