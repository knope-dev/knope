use log::error;
use miette::Diagnostic;
use thiserror::Error;

use super::{semver::Version, PackageName};
use crate::{
    dry_run::DryRun,
    fs,
    integrations::git::{self, create_tag},
};

pub(crate) fn tag_name(version: &Version, package_name: Option<&PackageName>) -> String {
    let prefix = package_name
        .as_ref()
        .map_or_else(|| "v".to_string(), |name| format!("{name}/v"));
    format!("{prefix}{version}")
}

pub(crate) fn release(
    dry_run_stdout: DryRun,
    version: &Version,
    package_name: Option<&PackageName>,
) -> Result<(), Error> {
    let tag = tag_name(version, package_name);

    create_tag(dry_run_stdout, tag).map_err(Error::from)
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(#[from] git::Error),
}
