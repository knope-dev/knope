use knope_versioning::{
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
            let current = package.take_version(&state.all_git_tags);
            let (version, go_versioning) = if let Some(version) = package.override_version.clone() {
                (version, GoVersioning::BumpMajor)
            } else {
                let version = current.bump(rule)?;
                (version, package.go_versioning)
            };
            let is_prerelease = version.is_prerelease();
            let actions = package.write_version(version, go_versioning)?;
            package.pending_actions =
                execute_prepare_actions(run_type.of(actions), is_prerelease, false)?;
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
    UpdatePackageVersion(#[from] knope_versioning::SetError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
}
