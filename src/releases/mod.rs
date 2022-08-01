pub(crate) use conventional_commits::update_project_from_conventional_commits as prepare_release;

use crate::state::Release::Prepared;
use crate::step::StepError;
use crate::RunType;

pub(crate) use self::package::{find_packages, suggested_package_toml, PackageConfig};
pub(crate) use self::semver::bump_version_and_update_state as bump_version;
pub(crate) use self::semver::{get_version, Rule};

mod cargo;
mod changelog;
mod conventional_commits;
mod git;
mod github;
mod package;
mod package_json;
mod pyproject;
mod semver;

#[derive(Clone, Debug)]
pub(crate) struct Release {
    pub(crate) version: ::semver::Version,
    pub(crate) changelog: String,
}

/// Create a release for the package.
///
/// If GitHub config is present, this creates a GitHub release. Otherwise, it tags the Git repo.
pub(crate) fn release(run_type: RunType) -> Result<RunType, StepError> {
    let (state, dry_run_stdout) = run_type.decompose();

    let release = match state.release.clone() {
        Prepared(release) => release,
        _ => return Err(StepError::ReleaseNotPrepared),
    };

    let github_config = state.github_config.clone();
    if let Some(github_config) = github_config {
        github::release(state, dry_run_stdout, &github_config, &release)
    } else {
        git::release(state, dry_run_stdout, &release)
    }
}
