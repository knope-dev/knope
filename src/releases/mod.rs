use std::{collections::BTreeMap, fmt, fmt::Display};

use ::changesets::PackageChange;
use time::{macros::format_description, OffsetDateTime};

pub(crate) use self::{
    changesets::{create_change_file, ChangeType},
    git::{get_current_versions_from_tag, tag_name},
    package::{
        find_packages, suggested_package_toml, ChangelogSectionSource, Package, PackageName,
    },
    semver::{bump_version_and_update_state as bump_version, Rule},
};
use crate::{
    releases::semver::{PreVersion, StableVersion, Version},
    step::StepError,
    RunType,
};

mod cargo;
pub(crate) mod changelog;
mod changesets;
mod conventional_commits;
pub(crate) mod git;
mod github;
pub(crate) mod go;
mod package;
mod package_json;
mod pyproject;
pub(crate) mod semver;
pub(crate) mod versioned_file;

use conventional_commits::ConventionalCommit;
pub(crate) use non_empty_map::PrereleaseMap;

use crate::{
    releases::conventional_commits::add_releases_from_conventional_commits, step::PrepareRelease,
    workflow::Verbose,
};

pub(crate) fn prepare_release(
    run_type: RunType,
    prepare_release: &PrepareRelease,
    verbose: Verbose,
) -> Result<RunType, StepError> {
    let (mut state, mut dry_run_stdout) = match run_type {
        RunType::DryRun { state, stdout } => (state, Some(stdout)),
        RunType::Real(state) => (state, None),
    };
    if state.packages.is_empty() {
        return Err(StepError::no_defined_packages_with_help());
    }
    let PrepareRelease { prerelease_label } = prepare_release;
    state.packages = add_releases_from_conventional_commits(state.packages)
        .and_then(|packages| changesets::add_releases_from_changeset(packages, &mut dry_run_stdout))
        .and_then(|packages| {
            packages
                .into_iter()
                .map(|package| {
                    package.write_release(prerelease_label, &mut dry_run_stdout, verbose)
                })
                .collect()
        })?;

    if let Some(stdout) = dry_run_stdout {
        Ok(RunType::DryRun { state, stdout })
    } else if state
        .packages
        .iter()
        .filter(|package| package.prepared_release.is_some())
        .count()
        == 0
    {
        Err(StepError::NoRelease)
    } else {
        Ok(RunType::Real(state))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Release {
    pub(crate) new_changelog: String,
    pub(crate) new_version: Version,
    date: OffsetDateTime,
}

impl Release {
    pub(crate) fn new(changelog: String, version: Version) -> Release {
        Release {
            new_changelog: changelog,
            new_version: version,
            date: OffsetDateTime::now_utc(),
        }
    }

    pub(crate) fn title(&self) -> Result<String, StepError> {
        let format = format_description!("[year]-[month]-[day]");
        let date = self.date.format(&format)?;
        Ok(format!("{} ({})", self.new_version, date))
    }

    pub(crate) fn changelog_entry(&self) -> Result<String, StepError> {
        Ok(format!("## {}\n\n{}", self.title()?, self.new_changelog))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Change {
    ConventionalCommit(ConventionalCommit),
    ChangeSet(PackageChange),
}

impl Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Change::ConventionalCommit(commit) => write!(f, "{commit}"),
            Change::ChangeSet(change) => {
                write!(f, "{}", change.unique_id.to_file_name())
            }
        }
    }
}

impl Change {
    fn change_type(&self) -> ChangeType {
        match self {
            Change::ConventionalCommit(commit) => commit.change_type.clone(),
            Change::ChangeSet(change) => (&change.change_type).into(),
        }
    }

    fn summary(&self) -> String {
        match self {
            Change::ConventionalCommit(commit) => commit.message.clone(),
            Change::ChangeSet(change) => change.summary.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct CurrentVersions {
    pub(crate) stable: Option<StableVersion>,
    pub(crate) prereleases: Prereleases,
}

type Prereleases = BTreeMap<StableVersion, PrereleaseMap>;

mod non_empty_map {
    use std::collections::BTreeMap;

    use crate::releases::semver::{Label, Prerelease};

    #[derive(Clone, Debug, Eq, PartialEq)]
    /// Used to track the various pre-releases of a version, can never be empty
    pub(crate) struct PrereleaseMap(BTreeMap<Label, Prerelease>);

    impl PrereleaseMap {
        /// Create a new map, cannot be empty
        pub(crate) fn new(prerelease: Prerelease) -> Self {
            let mut map = BTreeMap::new();
            map.insert(prerelease.label.clone(), prerelease);
            Self(map)
        }

        #[allow(clippy::unwrap_used)] // Map is not allowed to be empty ever
        pub(crate) fn into_last(mut self) -> Prerelease {
            self.0
                .pop_last()
                .map(|(_label, prerelease)| prerelease)
                .unwrap()
        }

        pub(crate) fn insert(&mut self, prerelease: Prerelease) {
            self.0.insert(prerelease.label.clone(), prerelease);
        }

        pub(crate) fn get(&self, key: &Label) -> Option<&Prerelease> {
            self.0.get(key)
        }
    }
}

impl CurrentVersions {
    pub(crate) fn into_latest(mut self) -> Option<Version> {
        self.prereleases
            .pop_last()
            .map(|(stable_component, pres)| {
                let pre_component = pres.into_last();
                Version::Pre(PreVersion {
                    stable_component,
                    pre_component,
                })
            })
            .or_else(|| self.stable.map(Version::Stable))
    }

    /// Replace or insert the version in the correct location if it's newer than the current
    /// equivalent version. If the version is a newer stable version, it will update `stable`.
    /// If the version is a newer prerelease, it will overwrite the prerelease with
    /// the same stable component and label.
    pub(crate) fn update_version(&mut self, version: Version) {
        match version {
            Version::Stable(new) => {
                if let Some(existing) = &self.stable {
                    if existing >= &new {
                        return;
                    }
                }
                self.stable = Some(new);
            }
            Version::Pre(PreVersion {
                stable_component,
                pre_component,
            }) => {
                let recorded_pre = self
                    .prereleases
                    .get(&stable_component)
                    .and_then(|pres| pres.get(&pre_component.label));
                if let Some(recorded_pre) = recorded_pre {
                    if recorded_pre >= &pre_component {
                        return;
                    }
                }
                if let Some(labels) = self.prereleases.get_mut(&stable_component) {
                    labels.insert(pre_component);
                } else {
                    self.prereleases
                        .insert(stable_component, PrereleaseMap::new(pre_component));
                }
            }
        }
    }
}

impl From<StableVersion> for CurrentVersions {
    fn from(version: StableVersion) -> Self {
        Self {
            stable: Some(version),
            prereleases: BTreeMap::new(),
        }
    }
}

impl From<Version> for CurrentVersions {
    fn from(version: Version) -> Self {
        let mut new = Self::default();
        new.update_version(version);
        new
    }
}

/// Create a release for the package.
///
/// If GitHub config is present, this creates a GitHub release. Otherwise, it tags the Git repo.
pub(crate) fn release(run_type: RunType) -> Result<RunType, StepError> {
    let (mut state, mut dry_run_stdout) = run_type.decompose();

    for package in &state.packages {
        if let Some(release) = package.prepared_release.as_ref() {
            let github_config = state.github_config.clone();
            if let Some(github_config) = github_config {
                state.github = github::release(
                    &package.name,
                    release,
                    state.github,
                    &github_config,
                    dry_run_stdout.as_mut(),
                )?;
            } else {
                git::release(&mut dry_run_stdout, &release.new_version, &package.name)?;
            }
        }
    }

    if let Some(stdout) = dry_run_stdout {
        Ok(RunType::DryRun { stdout, state })
    } else {
        if state
            .packages
            .iter()
            .filter(|p| p.prepared_release.is_some())
            .count()
            == 0
        {
            return Err(StepError::ReleaseNotPrepared);
        }

        Ok(RunType::Real(state))
    }
}
