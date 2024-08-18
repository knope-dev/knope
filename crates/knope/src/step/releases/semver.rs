use knope_versioning::{
    package::{Bump, BumpError},
    semver::{PreReleaseNotFound, Rule},
    GoVersioning, VersionedFile,
};
use miette::Diagnostic;

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

    for package in &mut state.packages {
        let (bump, go_versioning) = if let Some(version) = package.override_version.clone() {
            (Bump::Manual(version), GoVersioning::BumpMajor)
        } else {
            (Bump::Rule(rule.clone()), package.go_versioning)
        };
        state.all_versioned_files =
            package
                .versioning
                .bump_version(bump, go_versioning, state.all_versioned_files)?;
    }
    let write_files = state
        .all_versioned_files
        .into_iter()
        .filter_map(VersionedFile::write)
        .flatten();
    execute_prepare_actions(run_type.of(write_files), false)?;
    state.all_versioned_files = Vec::new();
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
