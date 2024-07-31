use std::{collections::BTreeMap, iter::once, path::PathBuf};

use ::changesets::ChangeSet;
use itertools::Itertools;
use knope_versioning::{
    changes::CHANGESET_DIR, Action, CreateRelease, PreVersion, ReleaseTag, StableVersion, Version,
};
use miette::Diagnostic;
pub(crate) use non_empty_map::PrereleaseMap;
use tracing::debug;

pub(crate) use self::{
    changelog::Release,
    package::Package,
    semver::{bump_version_and_update_state, Rule},
};
use crate::{
    integrations::{
        git,
        git::{create_tag, get_current_versions_from_tags},
    },
    state::State,
    step::PrepareRelease,
    RunType,
};

pub(crate) mod changelog;
pub(crate) mod conventional_commits;
pub(crate) mod gitea;
pub(crate) mod github;
pub(crate) mod package;
pub(crate) mod semver;
pub(crate) mod versioned_file;

pub(crate) fn prepare_release(
    state: RunType<State>,
    prepare_release: &PrepareRelease,
) -> Result<RunType<State>, Error> {
    let (mut state, dry_run) = match state {
        RunType::DryRun(state) => (state, true),
        RunType::Real(state) => (state, false),
    };
    if state.packages.is_empty() {
        return Err(package::Error::NoDefinedPackages.into());
    }

    let changeset_path = PathBuf::from(CHANGESET_DIR);
    let changeset = if changeset_path.exists() {
        ChangeSet::from_directory(&changeset_path)?.into()
    } else {
        Vec::new()
    };

    state.packages = state
        .packages
        .into_iter()
        .map(|package| {
            package.prepare_release(prepare_release, &state.all_git_tags, &changeset, dry_run)
        })
        .try_collect()?;

    if dry_run {
        Ok(RunType::DryRun(state))
    } else if !prepare_release.allow_empty
        && state
            .packages
            .iter()
            .all(|package| package.pending_actions.is_empty())
    {
        Err(Error::NoRelease)
    } else {
        Ok(RunType::Real(state))
    }
}

pub(crate) fn bump_version(state: RunType<State>, rule: &Rule) -> Result<RunType<State>, Error> {
    bump_version_and_update_state(state, rule).map_err(Error::from)
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
    #[error(transparent)]
    #[diagnostic(
        code(changesets::could_not_read_changeset),
        help(
            "This could be a file-system issue or a problem with the formatting of a change file."
        )
    )]
    CouldNotReadChangeSet(#[from] ::changesets::LoadingError),
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
pub(crate) fn release(state: RunType<State>) -> Result<RunType<State>, Error> {
    let (run_type, mut state) = state.take();

    let mut releases = state
        .packages
        .iter()
        .filter(|package| !package.pending_actions.is_empty())
        .collect_vec();

    if releases.is_empty() {
        releases = state
            .packages
            .iter_mut()
            .filter_map(|package| {
                let release = find_prepared_release(package, &state.all_git_tags)?;
                package.pending_actions = package
                    .versioning
                    .clone()
                    .set_version(&release.version, package.go_versioning)
                    .unwrap_or_default()
                    .into_iter()
                    // If the changelog was already written for this release, we don't need to write _any_ files
                    .filter(|action| matches!(action, Action::AddTag { .. }))
                    .chain(once(Action::CreateRelease(release)))
                    .rev()
                    .collect();
                Some(&*package)
            })
            .collect();
    }

    let github_config = state.github_config.as_ref();
    let gitea_config = state.gitea_config.as_ref();
    for (package, action) in releases.iter().flat_map(|package| {
        package
            .pending_actions
            .iter()
            .map(move |action| (package, action))
    }) {
        let release = match action {
            Action::AddTag { tag } => {
                if !ReleaseTag::is_release_tag(tag, package.name()) {
                    create_tag(run_type.of(tag.as_str()))?;
                }
                continue;
            }
            Action::CreateRelease(release) => release,
            _ => continue,
        };
        let tag = ReleaseTag::new(&release.version, package.name());
        if let Some(github_config) = github_config {
            state.github = github::release(
                package.name(),
                release,
                run_type.of(state.github),
                github_config,
                package.assets.as_ref(),
                &tag,
            )?;
        }

        if let Some(gitea_config) = gitea_config {
            state.gitea = gitea::release(
                package.name(),
                release,
                run_type.of(state.gitea),
                gitea_config,
                &tag,
            )?;
        }

        // if neither is present, we fall back to just creating a tag
        if github_config.is_none() && gitea_config.is_none() {
            create_tag(run_type.of(tag.as_str()))?;
        }
    }

    Ok(run_type.of(state))
}

/// Given a package, figure out if there was a release prepared in a separate workflow. Basically,
/// if the package version is newer than the latest tag, there's a release to release!
fn find_prepared_release(package: &mut Package, all_tags: &[String]) -> Option<CreateRelease> {
    let current_version = package.get_version(all_tags).clone().into_latest()?;
    debug!("Searching for last package tag to determine if there's a release to release");
    let last_tag = CurrentVersions::into_latest(get_current_versions_from_tags(
        package.name().as_custom(),
        all_tags,
    ));
    let version_of_new_release = match last_tag {
        Some(last_tag) if last_tag != current_version => current_version,
        None => current_version,
        _ => return None,
    };
    package
        .changelog
        .as_ref()
        .and_then(|changelog| changelog.get_release(&version_of_new_release))
        .map(|release| CreateRelease {
            version: version_of_new_release,
            notes: release.body_at_h1(),
        })
}
