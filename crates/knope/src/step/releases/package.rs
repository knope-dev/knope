use std::{fmt, fmt::Display, path::PathBuf};

use changesets::PackageChange;
use indexmap::IndexMap;
use itertools::Itertools;
use knope_config::{Assets, InternalDependencyUpdate, changelog_section::convert_to_versioning};
use knope_versioning::{
    Action, GoVersioning, PackageNewError, VersionedFile, VersionedFileError,
    changes::{CHANGESET_DIR, Change},
    package::{BumpError, ChangeConfig, Name},
    release_notes::{ReleaseNotes, TimeError},
    semver::Version,
};
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use tracing::{debug, info};

use super::{conventional_commits, semver};
use crate::{
    config, fs,
    fs::{WriteType, read_to_string},
    integrations::git::{self, add_files},
    state::RunType,
    step::{PrepareRelease, releases::changelog::load_changelog},
};

#[derive(Clone, Debug)]
pub(crate) struct Package {
    pub(crate) versioning: knope_versioning::Package,
    /// Version manually set by the caller to use instead of the one determined by semantic rule
    pub(crate) override_version: Option<Version>,
    pub(crate) assets: Option<Assets>,
    pub(crate) go_versioning: GoVersioning,
    /// Bump policy when one of this package's internal monorepo dependencies releases.
    pub(crate) update_internal_dependencies: InternalDependencyUpdate,
    /// Explicitly-declared names of packages this package depends on, supplementing the
    /// relationships derived from `versioned_files`.
    pub(crate) internal_dependencies: Vec<String>,
    /// When `true`, route conventional commits to this package by the files they touched.
    pub(crate) track_paths: bool,
    /// Directories/files that belong to this package for path-based commit filtering. When
    /// `track_paths` is true and this is empty, parents of the package's `versioned_files`
    /// are used.
    pub(crate) paths: Vec<RelativePathBuf>,
}

impl Package {
    pub(crate) fn load(
        release_notes: &knope_config::ReleaseNotes,
        packages: Vec<config::Package>,
        git_tags: &[String],
    ) -> Result<(Vec<Self>, Vec<VersionedFile>), Error> {
        let versioned_files: Vec<VersionedFile> = packages
            .iter()
            .flat_map(|package| package.versioned_files.iter())
            .map(|path| {
                let content = read_to_string(path.to_pathbuf())?;
                VersionedFile::new(path, content, git_tags).map_err(Error::VersionedFile)
            })
            .try_collect()?;
        let packages = packages
            .into_iter()
            .map(|package| {
                Package::validate(package, release_notes.clone(), git_tags, &versioned_files)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((packages, versioned_files))
    }

    pub(crate) fn name(&self) -> &Name {
        &self.versioning.name
    }

    fn validate(
        package: config::Package,
        release_notes: knope_config::ReleaseNotes,
        git_tags: &[String],
        all_versioned_files: &[VersionedFile],
    ) -> Result<Self, Error> {
        if let Name::Custom(package_name) = &package.name {
            debug!("Loading package {package_name}");
        } else {
            debug!("Loading package");
        }
        let versioning = knope_versioning::Package::new(
            package.name,
            git_tags,
            package.versioned_files,
            all_versioned_files,
            ReleaseNotes {
                sections: convert_to_versioning(package.extra_changelog_sections),
                changelog: package.changelog.map(load_changelog).transpose()?,
                change_templates: release_notes.change_templates,
            },
            package.scopes,
        )?;
        Ok(Self {
            versioning,
            assets: package.assets,
            go_versioning: if package.ignore_go_major_versioning {
                GoVersioning::IgnoreMajorRules
            } else {
                GoVersioning::default()
            },
            override_version: None,
            update_internal_dependencies: package.update_internal_dependencies,
            internal_dependencies: package.internal_dependencies,
            track_paths: package.track_paths,
            paths: package.paths,
        })
    }

    pub(crate) fn gather_changes(
        &self,
        prepare_release: &PrepareRelease,
        all_tags: &[String],
        changeset: &[changesets::Release],
        global_ignore_conventional_commits: bool,
    ) -> Result<Vec<Change>, Error> {
        let ignore_conventional_commits = prepare_release.ignore_conventional_commits;

        // Emit deprecation warning if step-level setting is used
        if ignore_conventional_commits {
            tracing::warn!(
                "The `ignore_conventional_commits` option on the PrepareRelease step is deprecated. \
                 Use `ignore_conventional_commits` in the `[changes]` config section instead. \
                 Run `knope --upgrade` to automatically migrate your config."
            );
        }

        // Use step-level setting if present, otherwise use global setting
        let should_ignore = ignore_conventional_commits || global_ignore_conventional_commits;

        let commit_messages = if should_ignore {
            Vec::new()
        } else {
            let commits = conventional_commits::get_conventional_commits_after_last_release(
                &self.versioning.name,
                all_tags,
                prepare_release.prerelease_label.is_some(),
            )?;
            if self.track_paths {
                let territory = self.territory();
                debug!(
                    "track_paths enabled for {pkg}; territory: {territory:?}",
                    pkg = self.versioning.name,
                );
                commits
                    .into_iter()
                    .filter(|commit| commit_touches_any(commit, &territory))
                    .collect()
            } else {
                commits
            }
        };

        let changeset_dir = PathBuf::from(CHANGESET_DIR);

        // Get commit information for change files
        let change_files = {
            let change_paths: IndexMap<PathBuf, &PackageChange> = changeset
                .iter()
                .find(|release| release.package_name == self.versioning.name.as_ref())
                .map(|release| {
                    release
                        .changes
                        .iter()
                        .map(|change| (changeset_dir.join(change.unique_id.to_file_name()), change))
                        .collect()
                })
                .unwrap_or_default();

            let mut git_info = git::get_first_commits_for_files(
                change_paths.keys().map(PathBuf::as_path).collect(),
            );

            change_paths
                .iter()
                .map(|(path, package_change)| {
                    (*package_change, git_info.remove(path.as_path()).flatten())
                })
                .collect_vec()
        };

        Ok(self.versioning.get_changes(change_files, &commit_messages))
    }

    /// Compute the package's "territory" — the set of repo-relative directories/files used
    /// for path-based commit filtering. Falls back to parent directories of non-dependency
    /// `versioned_files` when no explicit `paths` were configured.
    fn territory(&self) -> Vec<RelativePathBuf> {
        if !self.paths.is_empty() {
            return self.paths.clone();
        }
        let mut out: Vec<RelativePathBuf> = Vec::new();
        for vf in self.versioning.versioned_files() {
            if vf.dependency.is_some() {
                continue;
            }
            let path = vf.as_path();
            let dir = path.parent().map_or_else(
                || RelativePathBuf::from(""),
                relative_path::RelativePath::to_relative_path_buf,
            );
            if !out.contains(&dir) {
                out.push(dir);
            }
        }
        out
    }

    pub(crate) fn apply_release(
        &mut self,
        changes: &[Change],
        prepare_release: &PrepareRelease,
        versioned_files: Vec<VersionedFile>,
    ) -> Result<(Vec<VersionedFile>, Vec<Action>), Error> {
        if changes.is_empty() && self.override_version.is_none() {
            return Ok((versioned_files, Vec::new()));
        }

        let change_config = match self.override_version.take() {
            Some(version) => ChangeConfig::Force(version),
            None => ChangeConfig::Calculate {
                prerelease_label: prepare_release.prerelease_label.clone(),
                go_versioning: self.go_versioning,
            },
        };

        self.versioning
            .apply_changes(changes, versioned_files, change_config)
            .map_err(Error::Bump)
    }
}

/// Returns `true` if any file changed by `commit` is inside any path in `territory`. An
/// empty territory matches no files (so the package's commits are effectively dropped — the
/// safe interpretation for "`track_paths` but nothing configured to track").
fn commit_touches_any(
    commit: &knope_versioning::changes::conventional_commit::Commit,
    territory: &[RelativePathBuf],
) -> bool {
    if territory.is_empty() {
        return false;
    }
    commit
        .files
        .iter()
        .any(|file| territory.iter().any(|root| path_is_within(file, root)))
}

/// Whether `file` is `root` or sits inside `root` (treating `root` as a directory prefix).
fn path_is_within(file: &RelativePathBuf, root: &RelativePathBuf) -> bool {
    if root.as_str().is_empty() {
        return true;
    }
    if file == root {
        return true;
    }
    let root_str = root.as_str().trim_end_matches('/');
    let file_str = file.as_str();
    file_str.starts_with(root_str) && file_str.as_bytes().get(root_str.len()).copied() == Some(b'/')
}

pub(crate) fn execute_prepare_actions(
    actions: RunType<impl Iterator<Item = Action>>,
    stage_to_git: bool,
) -> Result<Vec<Action>, git::Error> {
    let (run_type, actions) = actions.take();
    let mut remainder = Vec::new();
    let mut paths_to_stage = Vec::new();
    for action in actions {
        match action {
            Action::WriteToFile {
                path,
                content,
                diff,
            } => {
                let write_type = match run_type {
                    RunType::DryRun(()) => WriteType::DryRun(diff),
                    RunType::Real(()) => WriteType::Real(content),
                };
                fs::write(write_type, &path.to_path(""))?;
                paths_to_stage.push(path);
            }
            Action::RemoveFile { path } => {
                // Ignore errors since we remove changesets per-package
                fs::remove_file(run_type.of(&path.to_path(""))).ok();
                paths_to_stage.push(path);
            }
            Action::AddTag { .. } | Action::CreateRelease(_) => {
                remainder.push(action);
            }
        }
    }
    if stage_to_git {
        stage_changes_to_git(run_type.of(&paths_to_stage))?;
    }
    Ok(remainder)
}

fn stage_changes_to_git(paths: RunType<&[RelativePathBuf]>) -> Result<(), git::Error> {
    match paths {
        RunType::DryRun(paths) => {
            if paths.is_empty() {
                return Ok(());
            }
            info!("Would add files to git:");
            for path in paths {
                info!("  {path}");
            }
            Ok(())
        }
        RunType::Real(paths) => {
            if paths.is_empty() {
                Ok(())
            } else {
                add_files(paths)
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
impl Package {
    pub(crate) fn default() -> Self {
        Self {
            versioning: knope_versioning::Package::new(
                Name::Default,
                &[""],
                vec![
                    knope_versioning::VersionedFileConfig::new("Cargo.toml".into(), None, None)
                        .unwrap(),
                ],
                &[VersionedFile::new(
                    &knope_versioning::VersionedFileConfig::new("Cargo.toml".into(), None, None)
                        .unwrap(),
                    r#"
                [package]
                name = "knope"
                version = "0.1.0""#
                        .to_string(),
                    &[""],
                )
                .unwrap()],
                ReleaseNotes {
                    sections: knope_versioning::release_notes::Sections::default(),
                    changelog: None,
                    change_templates: Vec::new(),
                },
                None,
            )
            .unwrap(),
            override_version: None,
            assets: None,
            go_versioning: GoVersioning::default(),
            update_internal_dependencies: InternalDependencyUpdate::default(),
            internal_dependencies: Vec::new(),
            track_paths: false,
            paths: Vec::new(),
        }
    }
}

/// So we can use it in an interactive select
impl Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Time(#[from] TimeError),
    #[error("Could not serialize generated TOML")]
    #[diagnostic(
        code(releases::package::could_not_serialize_toml),
        help("This is a bug, please report it to https://github.com/knope-dev/knope")
    )]
    GeneratedTOML(#[from] toml::ser::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Semver(#[from] semver::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Bump(#[from] BumpError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(#[from] git::Error),
    #[error("No packages to operate on")]
    #[diagnostic(
        code(package::no_defined_packages),
        help(
            "There must be at least one package for Knope to work with, no supported package files were found in this directory."
        ),
        url("https://knope.tech/reference/config-file/packages/")
    )]
    NoDefinedPackages,
    #[error(transparent)]
    #[diagnostic(transparent)]
    VersionedFile(#[from] VersionedFileError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    UpdatePackageVersion(#[from] knope_versioning::SetError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    New(#[from] Box<PackageNewError>),
}
