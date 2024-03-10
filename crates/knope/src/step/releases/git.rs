use log::error;
use miette::Diagnostic;
use thiserror::Error;

use crate::{
    dry_run::DryRun,
    integrations::git::{self, create_tag},
};

pub(crate) fn release(dry_run_stdout: DryRun, tag: &str) -> Result<(), Error> {
    create_tag(dry_run_stdout, tag).map_err(Error::from)
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(#[from] git::Error),
}
