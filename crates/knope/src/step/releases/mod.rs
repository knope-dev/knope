use std::collections::BTreeMap;

use itertools::Itertools;
use knope_versioning::{Action, PreVersion, StableVersion, Version};
use miette::Diagnostic;
pub(crate) use non_empty_map::PrereleaseMap;

pub(crate) use self::{
    changelog::Release,
    changesets::create_change_file,
    package::{Package, PackageName},
    semver::{bump_version_and_update_state, Rule},
};
use crate::{
    integrations::{
        git,
        git::{create_tag, get_current_versions_from_tags},
    },
    step::PrepareRelease,
    workflow::Verbose,
    RunType,
};

pub(crate) mod changelog;
pub(crate) mod changesets;
pub(crate) mod conventional_commits;
pub(crate) mod gitea;
pub(crate) mod github;
pub(crate) mod package;
pub(crate) mod semver;
pub(crate) mod versioned_file;

pub(crate) fn prepare_release(
    run_type: RunType,
    prepare_release: &PrepareRelease,
) -> Result<RunType, Error> {
    let (mut state, mut dry_run_stdout) = match run_type {
        RunType::DryRun { state, stdout } => (state, Some(stdout)),
        RunType::Real(state) => (state, None),
    };
    if state.packages.is_empty() {
        return Err(package::Error::NoDefinedPackages.into());
    }
    let PrepareRelease {
        prerelease_label,
        allow_empty,
        ignore_conventional_commits,
    } = prepare_release;
    if !ignore_conventional_commits {
        state.packages = state
            .packages
            .into_iter()
            .map(|mut package| {
                conventional_commits::get_conventional_commits_after_last_stable_version(
                    &package.name,
                    package.versioning.scopes.as_ref(),
                    &package.versioning.changelog_sections,
                    state.verbose,
                    &state.all_git_tags,
                )
                .map(|pending_changes| {
                    package.pending_changes = pending_changes;
                    package
                })
            })
            .try_collect()?;
    }
    state.packages = changesets::add_releases_from_changeset(
        state.packages,
        prerelease_label.is_some(),
        &mut dry_run_stdout,
    )
    .map_err(Error::from)
    .and_then(|packages| {
        packages
            .into_iter()
            .map(|package| {
                package
                    .write_release(
                        prerelease_label,
                        &state.all_git_tags,
                        &mut dry_run_stdout,
                        state.verbose,
                    )
                    .map_err(Error::from)
            })
            .collect()
    })?;

    if let Some(stdout) = dry_run_stdout {
        Ok(RunType::DryRun { state, stdout })
    } else if !*allow_empty
        && state
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

pub(crate) fn bump_version(run_type: RunType, rule: &Rule) -> Result<RunType, Error> {
    bump_version_and_update_state(run_type, rule).map_err(Error::from)
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
        url("https://knope.tech/reference/config-file/steps/prepare-release/#errors"),
    )]
    NoRelease,
    #[error(transparent)]
    #[diagnostic(transparent)]
    Semver(#[from] semver::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(#[from] git::Error),
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
    Gitea(#[from] gitea::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Parse(#[from] changelog::ParseError),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct CurrentVersions {
    pub(crate) stable: Option<StableVersion>,
    pub(crate) prereleases: Prereleases,
}

type Prereleases = BTreeMap<StableVersion, PrereleaseMap>;

mod non_empty_map {
    use std::collections::BTreeMap;

    use knope_versioning::{Label, Prerelease};

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
pub(crate) fn release(run_type: RunType) -> Result<RunType, Error> {
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
                find_prepared_release(package, state.verbose, &state.all_git_tags).map(|release| {
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
    let gitea_config = state.gitea_config.clone();
    for package_to_release in releases {
        let tag = tag_name(
            &package_to_release.release.version,
            &package_to_release.package.name,
        );

        if let Some(github_config) = github_config.as_ref() {
            state.github = github::release(
                package_to_release.package.name.as_ref(),
                &package_to_release.release,
                state.github,
                github_config,
                &mut dry_run_stdout,
                package_to_release.package.assets.as_ref(),
                &tag,
            )?;
        }

        if let Some(ref gitea_config) = gitea_config {
            state.gitea = gitea::release(
                package_to_release.package.name.as_ref(),
                &package_to_release.release,
                state.gitea,
                gitea_config,
                &mut dry_run_stdout,
                &tag,
            )?;
        }

        // if neither is present, we fall back to just creating a tag
        if github_config.is_none() && gitea_config.is_none() {
            create_tag(&mut dry_run_stdout, &tag)?;
        }

        package_to_release
            .release
            .actions
            .iter()
            .filter_map(|action| match action {
                Action::AddTag {
                    tag: additional_tag,
                } if *additional_tag != tag => Some(additional_tag),
                _ => None,
            })
            .try_for_each(|additional_tag| create_tag(&mut dry_run_stdout, additional_tag))?;
    }

    if let Some(stdout) = dry_run_stdout {
        Ok(RunType::DryRun { stdout, state })
    } else {
        Ok(RunType::Real(state))
    }
}

/// The tag that a particular version should have for a particular package
pub(crate) fn tag_name(version: &Version, package_name: &Option<PackageName>) -> String {
    let prefix = tag_prefix(package_name);
    format!("{prefix}{version}")
}

/// The prefix for tags for a particular package
fn tag_prefix(package_name: &Option<PackageName>) -> String {
    package_name
        .as_ref()
        .map_or_else(|| "v".to_string(), |name| format!("{name}/v"))
}

struct PackageWithRelease {
    package: Package,
    release: Release,
}

/// Given a package, figure out if there was a release prepared in a separate workflow. Basically,
/// if the package version is newer than the latest tag, there's a release to release!
fn find_prepared_release(
    package: &Package,
    verbose: Verbose,
    all_tags: &[String],
) -> Result<Option<Release>, Error> {
    let Some(current_version) = package.version_from_files() else {
        return Ok(None);
    };
    if let Verbose::Yes = verbose {
        println!("Searching for last package tag to determine if there's a release to release");
    }
    let last_tag = CurrentVersions::into_latest(get_current_versions_from_tags(
        package.name.as_deref(),
        verbose,
        all_tags,
    ));
    let version_of_new_release = match last_tag {
        Some(last_tag) if last_tag != *current_version => current_version,
        None => current_version,
        _ => return Ok(None),
    };
    package
        .changelog
        .as_ref()
        .map(|changelog| {
            changelog.get_release(
                version_of_new_release,
                package.versioning.clone(),
                package.go_versioning,
            )
        })
        .transpose()
        .map(Option::flatten)
        .map_err(Error::from)
}
