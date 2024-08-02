use knope_versioning::{
    package::{Bump, BumpError},
    semver::{PreReleaseNotFound, Rule},
    GoVersioning,
};
use miette::Diagnostic;

use super::package::Package;
use crate::{
    fs, integrations::git, state::State, step::releases::package::execute_prepare_actions, RunType,
};

/// The implementation of [`crate::step::Step::BumpVersion`].
///
/// Bumps the version of every configured package using `rule`.
pub(crate) fn bump_version_and_update_state(
    state: RunType<State>,
    rule: &Rule,
) -> Result<RunType<State>, Error> {
    let (run_type, mut state) = state.take();

    state.packages = state
        .packages
        .into_iter()
        .map(|mut package| {
            let (bump, go_versioning) = if let Some(version) = package.override_version.clone() {
                (Bump::Manual(version), GoVersioning::BumpMajor)
            } else {
                (Bump::Rule(rule.clone()), package.go_versioning)
            };
            let actions = package.versioning.bump_version(bump, go_versioning)?;
            package.pending_actions = execute_prepare_actions(run_type.of(actions), false)?;
            Ok(package)
        })
        .collect::<Result<Vec<Package>, Error>>()?;
    Ok(run_type.of(state))
}
#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    InvalidPreReleaseVersion(#[from] PreReleaseNotFound),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(#[from] git::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    UpdatePackageVersion(#[from] BumpError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
}
