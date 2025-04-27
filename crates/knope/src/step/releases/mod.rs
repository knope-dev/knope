use std::path::PathBuf;

use changesets::ChangeSet;
use itertools::Itertools;
use knope_versioning::{
    Action, ReleaseTag, VersionedFile,
    changes::CHANGESET_DIR,
    package::Bump,
    release_notes::Release,
    semver::{PackageVersions, Rule},
};
use miette::Diagnostic;
use tracing::debug;

pub(crate) use self::{package::Package, semver::bump_version_and_update_state};
use crate::{
    RunType, fs,
    integrations::{git, git::create_tag},
    state::State,
    step::{PrepareRelease, releases::package::execute_prepare_actions},
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

    for package in &mut state.packages {
        let (all_versioned_files, actions) = package.prepare_release(
            prepare_release,
            &state.all_git_tags,
            state.all_versioned_files,
            &changeset,
        )?;
        state.all_versioned_files = all_versioned_files;
        state.pending_actions.extend(actions);
    }

    let actions = state
        .all_versioned_files
        .drain(..)
        .filter_map(VersionedFile::write)
        .flatten()
        .chain(state.pending_actions)
        .unique();

    state.pending_actions = execute_prepare_actions(run_type.of(actions), true)?;

    match run_type {
        RunType::DryRun(()) => Ok(RunType::DryRun(state)),
        RunType::Real(()) => {
            if !prepare_release.allow_empty && state.pending_actions.is_empty() {
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
        help(
            "The `PrepareRelease` step will not complete if no changes cause a package's version to be increased."
        ),
        url("https://knope.tech/reference/config-file/steps/prepare-release/#errors")
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

    if state.pending_actions.is_empty() {
        for package in &mut state.packages {
            let Some(release) = find_prepared_release(package, &state.all_git_tags) else {
                continue;
            };
            state
                .pending_actions
                .push(Action::CreateRelease(release.clone()));
            let go_tags = package
                .versioning
                .bump_version(
                    Bump::Manual(release.version.clone()),
                    package.go_versioning,
                    state.all_versioned_files.clone(),
                )
                .unwrap_or_default()
                .into_iter()
                .filter_map(|versioned_file| {
                    versioned_file
                        .write()?
                        .into_iter()
                        .find(|action| matches!(action, Action::AddTag { .. }))
                });
            state.pending_actions.extend(go_tags);
        }
    }

    let github_config = state.github_config.as_ref();
    let gitea_config = state.gitea_config.as_ref();
    for action in state.pending_actions.drain(..) {
        let release = match action {
            Action::AddTag { tag } => {
                if !state
                    .packages
                    .iter()
                    .any(|package| ReleaseTag::is_release_tag(&tag, package.name()))
                {
                    create_tag(run_type.of(tag.as_str()))?;
                }
                continue;
            }
            Action::CreateRelease(release) => release,
            _ => continue,
        };
        let tag = ReleaseTag::new(&release.version, &release.package_name);
        if let Some(github_config) = github_config {
            state.github = github::release(
                &release,
                run_type.of(state.github),
                github_config,
                state
                    .packages
                    .iter()
                    .find(|package| package.name() == &release.package_name)
                    .and_then(|package| package.assets.as_ref()),
                &tag,
            )?;
        }

        if let Some(gitea_config) = gitea_config {
            state.gitea = gitea::release(&release, run_type.of(state.gitea), gitea_config, &tag)?;
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
fn find_prepared_release(package: &mut Package, all_tags: &[String]) -> Option<Release> {
    let current_version = package.versioning.versions.clone().into_latest();
    debug!("Searching for last package tag to determine if there's a release to release");
    let last_tag = PackageVersions::from_tags(package.name().as_custom(), all_tags).into_latest();
    debug!("Last tag is {last_tag}");
    if last_tag == current_version {
        return None;
    }
    Some(package
        .versioning
        .release_notes
        .changelog
        .as_ref()
        .and_then(|changelog| changelog.get_release(&current_version, package.name()))
        .unwrap_or_else(|| {
            debug!("Release has previously been prepared without a changelog, will create a release with no notes.");
            Release {
                title: current_version.to_string(),
                version: current_version,
                notes: String::new(),
                package_name: package.name().clone(),
            }
        }))
}
