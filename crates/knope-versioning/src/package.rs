use std::{
    borrow::{Borrow, Cow},
    fmt,
    fmt::{Debug, Display},
    ops::Deref,
};

use changesets::Release;
#[cfg(feature = "miette")]
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::debug;

use crate::{
    action::Action,
    changes::{
        conventional_commit::changes_from_commit_messages, Change, ChangeSource, CHANGESET_DIR,
    },
    release_notes::{ReleaseNotes, TimeError},
    semver::{Label, PackageVersions, PreReleaseNotFound, Rule, StableRule, Version},
    versioned_file,
    versioned_file::{GoVersioning, Path, SetError, VersionedFile},
};

#[derive(Clone, Debug)]
pub struct Package {
    pub name: Name,
    pub versions: PackageVersions,
    versioned_files: Vec<Path>,
    pub release_notes: ReleaseNotes,
    scopes: Option<Vec<String>>,
}

impl Package {
    /// Try and combine a bunch of versioned files into one logical package.
    ///
    /// # Errors
    ///
    /// There must be at least one versioned file, and all files must have the same version.
    pub fn new<S: AsRef<str> + Debug>(
        name: Name,
        git_tags: &[S],
        versioned_files_tracked: Vec<Path>,
        all_versioned_files: &[VersionedFile],
        release_notes: ReleaseNotes,
        scopes: Option<Vec<String>>,
    ) -> Result<Self, NewError> {
        let mut first_versioned_file: Option<(&VersionedFile, Version)> = None;

        for path in &versioned_files_tracked {
            let versioned_file = all_versioned_files
                .iter()
                .find(|f| f.path() == path)
                .ok_or_else(|| NewError::NotFound(path.as_path()))?;
            if path.dependency.is_some() {
                continue; // It's okay for dependencies to be out of date
            }
            let version = versioned_file.version()?;
            debug!("{path} has version {version}", path = path.as_path());
            if let Some((first_versioned_file, first_version)) = first_versioned_file.as_ref() {
                if *first_version != version {
                    return Err(NewError::InconsistentVersions {
                        first_path: first_versioned_file.path().clone(),
                        first_version: first_version.clone(),
                        second_path: versioned_file.path().clone(),
                        second_version: version,
                    });
                }
            } else {
                first_versioned_file = Some((versioned_file, version));
            }
        }
        debug!("Looking for Git tags matching package name.");
        let mut versions = PackageVersions::from_tags(name.as_custom(), git_tags);

        if let Some((_, version_from_files)) = first_versioned_file {
            versions.update_version(version_from_files);
        }

        Ok(Self {
            name,
            versions,
            versioned_files: versioned_files_tracked,
            release_notes,
            scopes,
        })
    }

    /// Returns the actions that must be taken to set this package to the new version, along
    /// with the version it was set to.
    ///
    /// The version can either be calculated from a semver rule or specified manually.
    ///
    /// # Errors
    ///
    /// If the file is a `go.mod`, there are rules about what versions are allowed.
    ///
    /// If serialization of some sort fails, which is a bug, then this will return an error.
    ///
    /// If the [`Rule::Release`] is specified, but there is no current prerelease, that's an
    /// error too.
    pub fn bump_version(
        &mut self,
        bump: Bump,
        go_versioning: GoVersioning,
        versioned_files: Vec<VersionedFile>,
    ) -> Result<Vec<VersionedFile>, BumpError> {
        match bump {
            Bump::Manual(version) => {
                self.versions.update_version(version);
            }
            Bump::Rule(rule) => {
                self.versions.bump(rule)?;
            }
        };
        let version = self.versions.clone().into_latest();
        versioned_files
            .into_iter()
            .map(|f| {
                let path = self.versioned_files.iter().find(|path| *path == f.path());
                if let Some(path) = path {
                    f.set_version(&version, path.dependency.as_deref(), go_versioning)
                        .map_err(BumpError::SetError)
                } else {
                    Ok(f)
                }
            })
            .collect()
    }

    #[must_use]
    pub fn get_changes(&self, changeset: &[Release], commit_messages: &[String]) -> Vec<Change> {
        changes_from_commit_messages(
            commit_messages,
            self.scopes.as_ref(),
            &self.release_notes.sections,
        )
        .chain(Change::from_changesets(&self.name, changeset))
        .collect()
    }

    /// Apply changes to the package, updating the internal version and returning the list of
    /// actions to take to complete the changes.
    ///
    /// # Errors
    ///
    /// If the file is a `go.mod`, there are rules about what versions are allowed.
    ///
    /// If serialization of some sort fails, which is a bug, then this will return an error.
    pub fn apply_changes(
        &mut self,
        changes: &[Change],
        versioned_files: Vec<VersionedFile>,
        config: ChangeConfig,
    ) -> Result<(Vec<VersionedFile>, Vec<Action>), BumpError> {
        if let Name::Custom(package_name) = &self.name {
            debug!("Determining new version for {package_name}");
        }

        let updated = match config {
            ChangeConfig::Force(version) => {
                debug!("Using overridden version {version}");
                self.bump_version(
                    Bump::Manual(version),
                    GoVersioning::BumpMajor,
                    versioned_files,
                )?
            }
            ChangeConfig::Calculate {
                prerelease_label,
                go_versioning,
            } => {
                let stable_rule = StableRule::from(changes);
                let rule = if let Some(pre_label) = prerelease_label {
                    Rule::Pre {
                        label: pre_label.clone(),
                        stable_rule,
                    }
                } else {
                    stable_rule.into()
                };
                self.bump_version(Bump::Rule(rule), go_versioning, versioned_files)?
            }
        };
        let version = self.versions.clone().into_latest();
        let mut actions: Vec<Action> = changes
            .iter()
            .filter_map(|change| {
                if let ChangeSource::ChangeFile(unique_id) = &change.original_source {
                    if version.is_prerelease() {
                        None
                    } else {
                        Some(Action::RemoveFile {
                            path: RelativePathBuf::from(CHANGESET_DIR)
                                .join(unique_id.to_file_name()),
                        })
                    }
                } else {
                    None
                }
            })
            .collect();

        actions.extend(
            self.release_notes
                .create_release(version, changes, &self.name)?,
        );

        Ok((updated, actions))
    }
}

pub enum ChangeConfig {
    Force(Version),
    Calculate {
        prerelease_label: Option<Label>,
        go_versioning: GoVersioning,
    },
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum NewError {
    #[error("Found inconsistent versions in package: {first_path} had {first_version} and {second_path} had {second_version}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code = "knope_versioning::inconsistent_versions",
            url = "https://knope.tech/reference/concepts/package/#version",
            help = "All files in a package must have the same version"
        )
    )]
    InconsistentVersions {
        first_path: RelativePathBuf,
        first_version: Version,
        second_path: RelativePathBuf,
        second_version: Version,
    },
    #[error("Versioned file not found: {0}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code = "knope_versioning::package::versioned_file_not_found",
            help = "this is likely a bug, please report it",
            url = "https://github.com/knope-dev/knope/issues/new",
        )
    )]
    NotFound(RelativePathBuf),
    #[error("Packages must have at least one versioned file")]
    NoPackages,
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    VersionedFile(#[from] versioned_file::Error),
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Name {
    Custom(String),
    #[default]
    Default,
}

impl Name {
    const DEFAULT: &'static str = "default";

    #[must_use]
    pub fn as_custom(&self) -> Option<&str> {
        match self {
            Self::Custom(name) => Some(name),
            Self::Default => None,
        }
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Custom(name) => write!(f, "{name}"),
            Self::Default => write!(f, "{}", Self::DEFAULT),
        }
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        match self {
            Self::Custom(name) => name,
            Self::Default => Self::DEFAULT,
        }
    }
}

impl Deref for Name {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Custom(name) => name,
            Self::Default => Self::DEFAULT,
        }
    }
}

impl From<&str> for Name {
    fn from(name: &str) -> Self {
        Self::Custom(name.to_string())
    }
}

impl From<String> for Name {
    fn from(name: String) -> Self {
        Self::Custom(name)
    }
}

impl From<Cow<'_, str>> for Name {
    fn from(name: Cow<str>) -> Self {
        Self::Custom(name.into_owned())
    }
}

impl Borrow<str> for Name {
    fn borrow(&self) -> &str {
        match self {
            Self::Custom(name) => name,
            Self::Default => Self::DEFAULT,
        }
    }
}

impl PartialEq<String> for Name {
    fn eq(&self, str: &String) -> bool {
        str == self.as_ref()
    }
}

pub enum Bump {
    Manual(Version),
    Rule(Rule),
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum BumpError {
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    SetError(#[from] SetError),
    #[error(transparent)]
    PreReleaseNotFound(#[from] PreReleaseNotFound),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    Time(#[from] TimeError),
}
