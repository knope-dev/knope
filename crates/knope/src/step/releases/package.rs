use std::{fmt, fmt::Display, path::PathBuf};

use itertools::Itertools;
use knope_config::changelog_section::convert_to_versioning;
use knope_versioning::{
    changelog::Sections,
    changes::Change,
    package::Name,
    semver::{PackageVersions, PreReleaseNotFound, Rule, StableRule, Version},
    Action, CreateRelease, GoVersioning, PackageNewError, VersionedFile, VersionedFileError,
};
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::{changelog, changelog::Changelog, conventional_commits, semver, Release};
use crate::{
    config, fs,
    fs::{read_to_string, WriteType},
    integrations::git::{self, add_files},
    state::RunType,
    step::{releases::changelog::HeaderLevel, PrepareRelease},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Package {
    pub(crate) version: Option<PackageVersions>,
    pub(crate) versioning: knope_versioning::Package,
    pub(crate) changelog: Option<Changelog>,
    pub(crate) pending_changes: Vec<Change>,
    pub(crate) pending_actions: Vec<Action>,
    /// Version manually set by the caller to use instead of the one determined by semantic rule
    pub(crate) override_version: Option<Version>,
    pub(crate) assets: Option<Vec<Asset>>,
    pub(crate) go_versioning: GoVersioning,
}

impl Package {
    pub(crate) fn load(
        packages: Vec<config::Package>,
        git_tags: &[String],
    ) -> Result<Vec<Self>, Error> {
        let packages = packages
            .into_iter()
            .map(|package| Package::validate(package, git_tags))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(packages)
    }

    pub(crate) fn name(&self) -> &Name {
        &self.versioning.name
    }

    /// Get the current version of a package determined by the last tag for the package _and_ the
    /// version in versioned files. The version from files takes precedent over version from tag.
    ///
    /// This is cached, so anything that mutates version is expected to update it here as well!
    pub(crate) fn get_version(&mut self, all_tags: &[String]) -> &PackageVersions {
        if self.version.is_none() {
            debug!("Looking for Git tags matching package name.");
            let mut versions = PackageVersions::from_tags(self.name().as_custom(), all_tags);

            if let Some(version_from_files) = self.versioning.get_version() {
                versions.update_version(version_from_files.clone());
            }

            self.version = Some(versions);
        }
        #[allow(clippy::unwrap_used)] // This was just inserted up above!
        self.version.as_ref().unwrap()
    }

    /// Like [`Self::get_version`], but removes from (or never stores in) cache.
    pub(crate) fn take_version(&mut self, all_tags: &[String]) -> PackageVersions {
        self.get_version(all_tags);
        self.version.take().unwrap_or_default()
    }

    fn validate(package: config::Package, git_tags: &[String]) -> Result<Self, Error> {
        if let Name::Custom(package_name) = &package.name {
            debug!("Loading package {package_name}");
        } else {
            debug!("Loading package");
        }
        let versioned_files: Vec<VersionedFile> = package
            .versioned_files
            .iter()
            .map(|path| {
                let content = read_to_string(path.to_pathbuf())?;
                VersionedFile::new(path, content, git_tags).map_err(Error::VersionedFile)
            })
            .try_collect()?;
        for versioned_file in &versioned_files {
            debug!(
                "{} has version {}",
                versioned_file.path(),
                versioned_file.version(),
            );
        }
        let versioning = knope_versioning::Package::new(
            package.name,
            versioned_files,
            convert_to_versioning(package.extra_changelog_sections),
            package.scopes,
        )?;
        Ok(Self {
            versioning,
            changelog: package.changelog.map(TryInto::try_into).transpose()?,
            assets: package.assets,
            go_versioning: if package.ignore_go_major_versioning {
                GoVersioning::IgnoreMajorRules
            } else {
                GoVersioning::default()
            },
            pending_changes: Vec::new(),
            pending_actions: Vec::new(),
            override_version: None,
            version: None,
        })
    }

    pub(crate) fn prepare_release(
        mut self,
        run_type: RunType<()>,
        prepare_release: &PrepareRelease,
        all_tags: &[String],
        changeset: &[changesets::Release],
    ) -> Result<Package, Error> {
        let PrepareRelease {
            prerelease_label,
            ignore_conventional_commits,
            ..
        } = prepare_release;

        let commit_messages = if *ignore_conventional_commits {
            Vec::new()
        } else {
            conventional_commits::get_conventional_commits_after_last_stable_version(
                &self.versioning.name,
                self.versioning.scopes.as_ref(),
                all_tags,
            )?
        };
        let changes = self.versioning.get_changes(changeset, &commit_messages);

        if changes.is_empty() {
            return Ok(self);
        }

        if let Name::Custom(package_name) = &self.versioning.name {
            debug!("Determining new version for {package_name}");
        }

        let mut pending_actions = self.versioning.apply_changes(&changes);

        let versions = self.take_version(all_tags);
        let (new_version, go_versioning) = if let Some(version) = self.override_version.take() {
            debug!("Using overridden version {version}");
            (version, GoVersioning::BumpMajor)
        } else {
            let stable_rule = StableRule::from(&changes);
            let rule = if let Some(pre_label) = prerelease_label {
                Rule::Pre {
                    label: pre_label.clone(),
                    stable_rule,
                }
            } else {
                stable_rule.into()
            };
            let version = versions.clone().bump(&rule)?;
            (version, self.go_versioning)
        };

        let actions = self.write_version(new_version.clone(), go_versioning)?;
        pending_actions.extend(actions);
        let is_prerelease = new_version.is_prerelease();
        let (actions, changelog) = make_release(
            self.changelog,
            &self.versioning.changelog_sections,
            &changes,
            new_version,
        )?;
        pending_actions.extend(actions);
        self.changelog = changelog;

        self.pending_actions =
            execute_prepare_actions(run_type.of(pending_actions), is_prerelease, true)?;

        Ok(self)
    }

    /// Consumes a [`Package`], writing it back to the file it came from. Returns the new version
    /// that was written. Adds all modified package files to Git.
    ///
    /// If `dry_run` is `true`, the version won't be written to any files.
    pub(crate) fn write_version(
        &mut self,
        version: Version,
        go_versioning: GoVersioning,
    ) -> Result<Vec<Action>, knope_versioning::SetError> {
        let actions = self
            .versioning
            .clone()
            .set_version(&version, go_versioning)?;
        self.version = Some(version.into());
        Ok(actions)
    }
}

/// Adds content from `release` to `Self::changelog` if it exists.
fn make_release(
    mut changelog: Option<Changelog>,
    changelog_sections: &Sections,
    changes: &[Change],
    version: Version,
) -> Result<(Vec<Action>, Option<Changelog>), crate::step::releases::changelog::Error> {
    let release_header_level = changelog
        .as_ref()
        .map_or(HeaderLevel::H1, |changelog| changelog.release_header_level);
    let release = Release::new(&version, changes, changelog_sections, release_header_level)?;
    let mut pending_actions = Vec::new();

    changelog = if let Some(changelog) = changelog {
        let (changelog, new_changes) = changelog.with_release(&release);
        pending_actions.push(Action::WriteToFile {
            path: changelog.path.clone(),
            content: changelog.content.clone(),
            diff: format!("\n{new_changes}\n"),
        });
        Some(changelog)
    } else {
        None
    };
    pending_actions.push(Action::CreateRelease(CreateRelease {
        version,
        notes: release.body_at_h1(),
    }));
    Ok((pending_actions, changelog))
}

pub(crate) fn execute_prepare_actions(
    actions: RunType<Vec<Action>>,
    is_prerelease: bool,
    stage_to_git: bool,
) -> Result<Vec<Action>, git::Error> {
    let (run_type, actions) = actions.take();
    let mut remainder = Vec::with_capacity(actions.len());
    let mut paths_to_stage = Vec::with_capacity(actions.len());
    for action in actions {
        match action {
            Action::WriteToFile {
                path,
                content,
                diff,
            } => {
                // TODO: What if two packages wanted to write to the same file? Like changelog?
                let write_type = match run_type {
                    RunType::DryRun(()) => WriteType::DryRun(diff),
                    RunType::Real(()) => WriteType::Real(content),
                };
                fs::write(write_type, &path.to_path(""))?;
                paths_to_stage.push(path);
            }
            Action::RemoveFile { path } => {
                if is_prerelease {
                    continue; // Don't remove changesets for prereleases
                }
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
                vec![VersionedFile::new(
                    &knope_versioning::VersionedFilePath::new("Cargo.toml".into()).unwrap(),
                    r#"
                [package]
                name = "knope"
                version = "0.1.0""#
                        .to_string(),
                    &[""],
                )
                .unwrap()],
                Sections::default(),
                None,
            )
            .unwrap(),
            changelog: None,
            pending_changes: vec![],
            pending_actions: vec![],
            override_version: None,
            assets: None,
            go_versioning: GoVersioning::default(),
            version: None,
        }
    }
}

/// So we can use it in an interactive select
impl Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Asset {
    pub(crate) path: PathBuf,
    name: Option<String>,
}

impl Asset {
    pub(crate) fn name(&self) -> Result<String, AssetNameError> {
        if let Some(name) = &self.name {
            Ok(name.clone())
        } else {
            self.path
                .file_name()
                .ok_or(AssetNameError {
                    path: self.path.clone(),
                })
                .map(|name| name.to_string_lossy().into_owned())
        }
    }
}

#[derive(Debug, Diagnostic, thiserror::Error)]
#[error("No asset name set, and name could not be determined from path {path}")]
pub(crate) struct AssetNameError {
    path: PathBuf,
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Changelog(#[from] changelog::Error),
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
    PreReleaseNotFound(#[from] PreReleaseNotFound),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(#[from] git::Error),
    #[error("No packages to operate on")]
    #[diagnostic(
        code(package::no_defined_packages),
        help("There must be at least one package for Knope to work with, no supported package files were found in this directory."),
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
    New(#[from] PackageNewError),
}
