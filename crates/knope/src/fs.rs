//! Proxies to FS utils that _either_ actually write to files or print to stdout (for dry runs).

use std::{
    fmt::Display,
    io,
    path::{Path, PathBuf},
};

use log::trace;
use miette::Diagnostic;
use thiserror::Error;

use crate::dry_run::DryRun;

/// Writes to a file if this is not a dry run, or prints just the diff to stdout if it is.
pub(crate) fn write<C: AsRef<[u8]> + Display>(
    dry_run: DryRun,
    diff: &str,
    path: &Path,
    contents: C,
) -> Result<(), Error> {
    if let Some(stdout) = dry_run {
        writeln!(
            stdout,
            "Would add the following to {}: {diff}",
            path.display()
        )
        .map_err(Error::Stdout)
    } else {
        trace!("Writing {} to {}", contents, path.display());
        std::fs::write(path, contents).map_err(|source| Error::Write {
            path: path.into(),
            source,
        })
    }
}

pub(crate) fn create_dir(dry_run: DryRun, path: &Path) -> Result<(), Error> {
    if let Some(stdout) = dry_run {
        writeln!(stdout, "Would create directory {}", path.display()).map_err(Error::Stdout)
    } else {
        trace!("Creating directory {}", path.display());
        std::fs::create_dir_all(path).map_err(|source| Error::Write {
            path: path.into(),
            source,
        })
    }
}

pub(crate) fn read_to_string(path: &Path) -> Result<String, Error> {
    std::fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.into(),
        source,
    })
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error("Error writing to {path}: {source}")]
    #[diagnostic(
        code(fs::write),
        help("Make sure you have permission to write to this file.")
    )]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Error reading from {path}: {source}")]
    #[diagnostic(
        code(fs::read),
        help("Make sure you have permission to read this file.")
    )]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Error writing to stdout: {0}")]
    Stdout(#[source] io::Error),
}
