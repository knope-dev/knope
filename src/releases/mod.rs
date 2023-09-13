use std::{collections::BTreeMap, fmt, fmt::Display};

use ::changesets::PackageChange;
use itertools::Itertools;
use miette::Diagnostic;
use time::{macros::format_description, OffsetDateTime};

pub(crate) use self::{
    changesets::{create_change_file, ChangeType},
    git::tag_name,
    package::{find_packages, ChangelogSectionSource, Package, PackageName},
    semver::{bump_version_and_update_state, Rule},
};
use crate::{
    releases::semver::{PreVersion, StableVersion, Version},
    RunType,
};

mod cargo;
pub(crate) mod changelog;
pub(crate) mod changesets;
mod conventional_commits;
pub(crate) mod git;
pub(crate) mod github;
pub(crate) mod go;
pub(crate) mod package;
mod package_json;
mod pyproject;
pub(crate) mod semver;
pub(crate) mod versioned_file;

use conventional_commits::ConventionalCommit;
pub(crate) use non_empty_map::PrereleaseMap;

use crate::{
    dry_run::DryRun,
    git::get_current_versions_from_tags,
    releases::{
        conventional_commits::add_releases_from_conventional_commits, versioned_file::PackageFormat,
    },
    step::PrepareRelease,
    workflow::Verbose,
};

pub(crate) fn prepare_release(
    run_type: RunType,
    prepare_release: &PrepareRelease,
    verbose: Verbose,
) -> Result<RunType, Error> {
    let (mut state, mut dry_run_stdout) = match run_type {
        RunType::DryRun { state, stdout } => (state, Some(stdout)),
        RunType::Real(state) => (state, None),
    };
    if state.packages.is_empty() {
        return Err(package::Error::no_defined_packages_with_help().into());
    }
    let PrepareRelease { prerelease_label } = prepare_release;
    state.packages = add_releases_from_conventional_commits(state.packages, verbose)
        .map_err(Error::from)
        .and_then(|packages| {
            changesets::add_releases_from_changeset(packages, &mut dry_run_stdout)
                .map_err(Error::from)
        })
        .and_then(|packages| {
            packages
                .into_iter()
                .map(|package| {
                    package
                        .write_release(prerelease_label, &mut dry_run_stdout, verbose)
                        .map_err(Error::from)
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
        Err(Error::NoRelease)
    } else {
        Ok(RunType::Real(state))
    }
}

pub(crate) fn bump_version(
    run_type: RunType,
    rule: &Rule,
    verbose: Verbose,
) -> Result<RunType, Error> {
    bump_version_and_update_state(run_type, rule, verbose).map_err(Error::from)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Release {
    pub(crate) new_changelog: Option<String>,
    pub(crate) new_version: Version,
    date: OffsetDateTime,
}

impl Release {
    pub(crate) fn new(changelog: Option<String>, version: Version) -> Release {
        Release {
            new_changelog: changelog,
            new_version: version,
            date: OffsetDateTime::now_utc(),
        }
    }

    pub(crate) fn title(&self) -> Result<String, TimeError> {
        let format = format_description!("[year]-[month]-[day]");
        let date = self.date.format(&format)?;
        Ok(format!("{} ({})", self.new_version, date))
    }

    pub(crate) fn changelog_entry(&self) -> Result<Option<String>, TimeError> {
        self.title().map(|title| {
            self.new_changelog
                .as_ref()
                .map(|changelog| format!("## {title}\n\n{changelog}"))
        })
    }
}

#[derive(Debug, Diagnostic, thiserror::Error)]
#[error("Failed to format current time")]
#[diagnostic(
    code(releases::time_format),
    help("This is probably a bug with knope, please file an issue at https://github.com/knope-dev/knope")
)]
pub(crate) struct TimeError(#[from] time::error::Format);

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("No packages are ready to release")]
    #[diagnostic(
        code(releases::no_release),
        help("The `PrepareRelease` step will not complete if no changes cause a package's version to be increased."),
        url("https://knope-dev.github.io/knope/config/step/PrepareRelease.html"),
    )]
    NoRelease,
    #[error(transparent)]
    #[diagnostic(transparent)]
    Semver(#[from] semver::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    GitRelease(#[from] self::git::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(#[from] crate::git::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    ChangeSet(#[from] changesets::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Package(#[from] package::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    GitHub(#[from] github::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    ConventionalCommits(#[from] conventional_commits::Error),
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
pub(crate) fn release(run_type: RunType, verbose: Verbose) -> Result<RunType, Error> {
    let (mut state, mut dry_run_stdout) = run_type.decompose();

    let mut releases = state
        .packages
        .iter_mut()
        .filter_map(|package| {
            package
                .prepared_release
                .take()
                .map(|release| PackageWithRelease {
                    package: package.clone(),
                    release,
                })
        })
        .collect_vec();

    if releases.is_empty() {
        releases = state
            .packages
            .iter()
            .map(|package| {
                find_prepared_release(package, verbose).map(|release| {
                    release.map(|release| PackageWithRelease {
                        package: package.clone(),
                        release,
                    })
                })
            })
            .filter_map_ok(|stuff| stuff)
            .try_collect()?;
    }

    let github_config = state.github_config.clone();
    for package_to_release in releases {
        if let Some(github_config) = github_config.as_ref() {
            state.github = github::release(
                package_to_release.package.name.as_ref(),
                &package_to_release.release,
                state.github,
                github_config,
                &mut dry_run_stdout,
                package_to_release.package.assets.as_ref(),
            )?;
        } else {
            git::release(
                &mut dry_run_stdout,
                &package_to_release.release.new_version,
                package_to_release.package.name.as_ref(),
            )?;
        }
        add_go_mod_tags(&package_to_release, &mut dry_run_stdout)?;
    }

    if let Some(stdout) = dry_run_stdout {
        Ok(RunType::DryRun { stdout, state })
    } else {
        Ok(RunType::Real(state))
    }
}

struct PackageWithRelease {
    package: Package,
    release: Release,
}

/// Given a package, figure out if there was a release prepared in a separate workflow. Basically,
/// if the package version is newer than the latest tag, there's a release to release!
fn find_prepared_release(package: &Package, verbose: Verbose) -> Result<Option<Release>, Error> {
    let Some(current_version) = package.version_from_files(verbose)? else {
        return Ok(None);
    };
    let last_tag = get_current_versions_from_tags(package.name.as_deref(), verbose)
        .map(CurrentVersions::into_latest)?;
    let version_of_new_release = match last_tag {
        Some(last_tag) if last_tag != current_version => current_version,
        None => current_version,
        _ => return Ok(None),
    };
    Ok(Some(Release {
        new_changelog: package
            .changelog
            .as_ref()
            .and_then(|changelog| changelog.get_section(&version_of_new_release)),
        new_version: version_of_new_release,
        date: OffsetDateTime::now_utc(),
    }))
}

/// Go modules have their versions determined by a Git tag. They _also_ have a _piece_ of their
/// version in the `go.mod` file _sometimes_. For every other language, `PrepareRelease` updates
/// the version in the file that defines the version (e.g., Cargo.toml). Typically, consumers will
/// add a new Git commit _after_ `PrepareRelease`, before `Release`, so if we add the Go tag there,
/// it's in the wrong place. So the `Release` step needs to write the _right_ version.
fn add_go_mod_tags(
    package_with_release: &PackageWithRelease,
    dry_run: DryRun,
) -> Result<(), git::Error> {
    let PackageWithRelease { package, release } = package_with_release;
    let go_mods = package
        .versioned_files
        .iter()
        .filter(|versioned_file| matches!(versioned_file.format, PackageFormat::Go))
        .collect_vec();
    for go_mod in go_mods {
        go::create_version_tag(&go_mod.path, &release.new_version, dry_run)?;
    }
    Ok(())
}
