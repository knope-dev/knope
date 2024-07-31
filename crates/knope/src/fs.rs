//! Proxies to FS utils that _either_ actually write to files or print to stdout (for dry runs).

use std::{
    fmt::Display,
    io,
    path::{Path, PathBuf},
};

use miette::Diagnostic;
use thiserror::Error;
use tracing::{info, trace};

use crate::state::RunType;

/// Writes to a file if this is not a dry run, or prints just the diff to stdout if it is.
pub(crate) fn write<C: AsRef<[u8]> + Display, Diff: Display>(
    to_write: WriteType<C, Diff>,
    path: &Path,
) -> Result<(), Error> {
    match to_write {
        WriteType::DryRun(diff) => {
            info!("Would add the following to {}: {diff}", path.display());
            Ok(())
        }
        WriteType::Real(contents) => {
            trace!("Writing {} to {}", contents, path.display());
            std::fs::write(path, contents).map_err(|source| Error::Write {
                path: path.into(),
                source,
            })
        }
    }
}

pub(crate) enum WriteType<Real, DryRun> {
    Real(Real),
    DryRun(DryRun),
}

pub(crate) fn create_dir(path: RunType<&Path>) -> Result<(), Error> {
    match path {
        RunType::DryRun(path) => {
            info!("Would create directory {}", path.display());
            Ok(())
        }
        RunType::Real(path) => {
            trace!("Creating directory {}", path.display());
            std::fs::create_dir_all(path).map_err(|source| Error::Write {
                path: path.into(),
                source,
            })
        }
    }
}

pub(crate) fn read_to_string<P: AsRef<Path> + Into<PathBuf>>(path: P) -> Result<String, Error> {
    std::fs::read_to_string(path.as_ref()).map_err(|source| Error::Read {
        path: path.into(),
        source,
    })
}

pub(crate) fn remove_file(path: RunType<&Path>) -> Result<(), Error> {
    match path {
        RunType::DryRun(path) => {
            info!("Would delete {}", path.display());
            Ok(())
        }
        RunType::Real(path) => {
            trace!("Removing file {}", path.display());
            std::fs::remove_file(path).map_err(|source| Error::Remove {
                path: path.into(),
                source,
            })
        }
    }
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
    #[error("Error removing {path}: {source}")]
    #[diagnostic(
        code(fs::remove),
        help("Make sure you have permission to write to this file.")
    )]
    Remove {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}
