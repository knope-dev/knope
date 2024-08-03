use std::{iter::once, path::PathBuf};

use changesets::ChangeSet;
use itertools::Itertools;
use knope_versioning::{
    changes::CHANGESET_DIR,
    package::Bump,
    release_notes,
    semver::{PackageVersions, Rule},
    Action, CreateRelease, ReleaseTag,
};
use miette::Diagnostic;
use tracing::debug;

pub(crate) use self::{package::Package, semver::bump_version_and_update_state};
use crate::{
    fs,
    integrations::{git, git::create_tag},
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

pub(crate) fn prepare_release(
    state: RunType<State>,
    prepare_release: &PrepareRelease,
) -> Result<RunType<State>, Error> {
    let (run_type, mut state) = state.take();
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
            package.prepare_release(run_type, prepare_release, &state.all_git_tags, &changeset)
        })
        .try_collect()?;

    match run_type {
        RunType::DryRun(()) => Ok(RunType::DryRun(state)),
        RunType::Real(()) => {
            if !prepare_release.allow_empty
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
    }
}

pub(crate) fn bump_version(state: RunType<State>, rule: &Rule) -> Result<RunType<State>, Error> {
    bump_version_and_update_state(state, rule).map_err(Error::from)
}

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
    Parse(#[from] release_notes::ParseError),
    #[error(transparent)]
    #[diagnostic(
        code(changesets::could_not_read_changeset),
        help(
            "This could be a file-system issue or a problem with the formatting of a change file."
        )
    )]
    CouldNotReadChangeSet(#[from] changesets::LoadingError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
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
                    .bump_version(Bump::Manual(release.version.clone()), package.go_versioning)
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
    let current_version = package.versioning.versions.clone().into_latest();
    debug!("Searching for last package tag to determine if there's a release to release");
    let last_tag = PackageVersions::from_tags(package.name().as_custom(), all_tags).into_latest();
    if last_tag == current_version {
        return None;
    }
    package
        .versioning
        .release_notes
        .changelog
        .as_ref()
        .and_then(|changelog| changelog.get_release(&current_version))
        .map(|release| CreateRelease {
            version: current_version,
            notes: release.body_at_h1(),
        })
}
