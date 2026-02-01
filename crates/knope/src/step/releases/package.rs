use std::{fmt, fmt::Display, path::PathBuf};

use changesets::PackageChange;
use indexmap::IndexMap;
use itertools::Itertools;
use knope_config::{Assets, changelog_section::convert_to_versioning};
use knope_versioning::{
    Action, GoVersioning, PackageNewError, VersionedFile, VersionedFileError,
    changes::CHANGESET_DIR,
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
        })
    }

    pub(crate) fn prepare_release(
        &mut self,
        prepare_release: &PrepareRelease,
        all_tags: &[String],
        versioned_files: Vec<VersionedFile>,
        changeset: &[changesets::Release],
        global_ignore_conventional_commits: bool,
    ) -> Result<(Vec<VersionedFile>, Vec<Action>), Error> {
        let PrepareRelease {
            prerelease_label,
            ignore_conventional_commits,
            ..
        } = prepare_release;

        // Emit deprecation warning if step-level setting is used
        if *ignore_conventional_commits {
            tracing::warn!(
                "The `ignore_conventional_commits` option on the PrepareRelease step is deprecated. \
                 Use the top-level `ignore_conventional_commits` config setting instead. \
                 Run `knope --upgrade` to automatically migrate your config."
            );
        }

        // Use step-level setting if present, otherwise use global setting
        let should_ignore = *ignore_conventional_commits || global_ignore_conventional_commits;

        let commit_messages = if should_ignore {
            Vec::new()
        } else {
            conventional_commits::get_conventional_commits_after_last_stable_version(
                &self.versioning.name,
                all_tags,
            )?
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

        let changes = self.versioning.get_changes(change_files, &commit_messages);

        if changes.is_empty() && self.override_version.is_none() {
            return Ok((versioned_files, Vec::new()));
        }

        let change_config = match self.override_version.take() {
            Some(version) => ChangeConfig::Force(version),
            None => ChangeConfig::Calculate {
                prerelease_label: prerelease_label.clone(),
                go_versioning: self.go_versioning,
            },
        };

        self.versioning
            .apply_changes(&changes, versioned_files, change_config)
            .map_err(Error::Bump)
    }
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
