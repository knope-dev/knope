use std::collections::BTreeMap;

pub(crate) use conventional_commits::update_project_from_conventional_commits as prepare_release;

use crate::releases::semver::{Label, Prerelease, Version};
use crate::state::Release::{Bumped, Prepared};
use crate::step::StepError;
use crate::RunType;

pub(crate) use self::git::{get_current_versions_from_tag, tag_name};
pub(crate) use self::package::{find_packages, suggested_package_toml, Package};
pub(crate) use self::semver::bump_version_and_update_state as bump_version;
pub(crate) use self::semver::{get_version, Rule};

mod cargo;
mod changelog;
mod conventional_commits;
mod git;
mod github;
mod go;
mod package;
mod package_json;
mod pyproject;
pub(crate) mod semver;

#[derive(Clone, Debug)]
pub(crate) struct Release {
    pub(crate) version: Version,
    pub(crate) changelog: String,
    pub(crate) package_name: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct CurrentVersions {
    pub(crate) stable: Option<Version>,
    pub(crate) prereleases: Prereleases,
}

type Prereleases = BTreeMap<Version, BTreeMap<Label, Prerelease>>;

impl CurrentVersions {
    pub(crate) fn into_latest(mut self) -> Option<Version> {
        self.prereleases
            .pop_last()
            .map(|(version, mut pres)| {
                let mut version = version;
                version.pre = Some(
                    pres.pop_last()
                        .expect("This map is never allowed to be empty")
                        .1,
                );
                version
            })
            .or(self.stable)
    }

    pub(crate) fn insert_prerelease(&mut self, prerelease: Version) {
        let (stable_component, pre) = {
            let mut version = prerelease;
            let pre = version.pre.take().expect("This is a prerelease");
            (version, pre)
        };
        let recorded_pre = self
            .prereleases
            .get(&stable_component)
            .and_then(|pres| pres.get(&pre.label));
        if let Some(recorded_pre) = recorded_pre {
            if recorded_pre >= &pre {
                return;
            }
        }
        self.prereleases
            .entry(stable_component)
            .or_default()
            .insert(pre.label.clone(), pre);
    }

    pub(crate) fn replace_stable_if_newer(&mut self, version: Version) {
        if let Some(stable) = &self.stable {
            if stable >= &version {
                return;
            }
        }
        self.stable = Some(version);
    }
}

impl From<Version> for CurrentVersions {
    fn from(version: Version) -> Self {
        Self {
            stable: Some(version),
            prereleases: BTreeMap::new(),
        }
    }
}

/// Create a release for the package.
///
/// If GitHub config is present, this creates a GitHub release. Otherwise, it tags the Git repo.
pub(crate) fn release(run_type: RunType) -> Result<RunType, StepError> {
    let (mut state, mut dry_run_stdout) = run_type.decompose();

    for release in &state.releases {
        let prepared = match release {
            Prepared(release) => release,
            Bumped { .. } => return Err(StepError::ReleaseNotPrepared),
        };

        let github_config = state.github_config.clone();
        if let Some(github_config) = github_config {
            state.github = github::release(
                prepared,
                state.github,
                &github_config,
                dry_run_stdout.as_mut(),
            )?;
        } else {
            git::release(dry_run_stdout.as_mut(), prepared)?;
        }
    }

    if let Some(stdout) = dry_run_stdout {
        Ok(RunType::DryRun { stdout, state })
    } else {
        Ok(RunType::Real(state))
    }
}
