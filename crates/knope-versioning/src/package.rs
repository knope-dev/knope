use std::{
    borrow::{Borrow, Cow},
    fmt,
    fmt::{Debug, Display},
    ops::Deref,
};

use changesets::Release;
use itertools::Itertools;
#[cfg(feature = "miette")]
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::debug;

use crate::{
    action::Action,
    changelog::Sections as ChangelogSections,
    changes::{
        conventional_commit::changes_from_commit_messages, Change, ChangeSource, CHANGESET_DIR,
    },
    semver::{Label, PackageVersions, PreReleaseNotFound, Rule, StableRule, Version},
    versioned_file::{GoVersioning, SetError, VersionedFile},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Package {
    pub name: Name,
    pub versions: PackageVersions,
    versioned_files: Option<Vec<VersionedFile>>,
    pub changelog_sections: ChangelogSections,
    pub scopes: Option<Vec<String>>,
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
        versioned_files: Vec<VersionedFile>,
        changelog_sections: ChangelogSections,
        scopes: Option<Vec<String>>,
    ) -> Result<Self, NewError> {
        if let Some(first) = versioned_files.first() {
            if let Some(conflict) = versioned_files
                .iter()
                .find(|f| f.version() != first.version())
            {
                return Err(NewError::InconsistentVersions(
                    Box::new(first.clone()),
                    Box::new(conflict.clone()),
                ));
            }
        }
        debug!("Looking for Git tags matching package name.");
        let mut versions = PackageVersions::from_tags(name.as_custom(), git_tags);

        if let Some(version_from_files) = versioned_files.first().map(VersionedFile::version) {
            versions.update_version(version_from_files.clone());
        }

        Ok(Self {
            name,
            versions,
            versioned_files: Some(versioned_files),
            changelog_sections,
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
    ) -> Result<Vec<Action>, BumpError> {
        let Some(versioned_files) = self.versioned_files.take() else {
            return Err(BumpError::PackageAlreadyBumped);
        };
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
            .map(|f| f.set_version(&version, go_versioning))
            .process_results(|iter| iter.flatten().collect())
            .map_err(BumpError::SetError)
    }

    #[must_use]
    pub fn get_changes(&self, changeset: &[Release], commit_messages: &[String]) -> Vec<Change> {
        changes_from_commit_messages(
            commit_messages,
            self.scopes.as_ref(),
            &self.changelog_sections,
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
        config: ChangeConfig,
    ) -> Result<Vec<Action>, BumpError> {
        if let Name::Custom(package_name) = &self.name {
            debug!("Determining new version for {package_name}");
        }

        let mut actions = match config {
            ChangeConfig::Force(version) => {
                debug!("Using overridden version {version}");
                self.bump_version(Bump::Manual(version), GoVersioning::BumpMajor)?
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
                self.bump_version(Bump::Rule(rule), go_versioning)?
            }
        };
        let pre_release = self.versions.latest_is_prerelease();
        actions.extend(changes.iter().filter_map(|change| {
            if let ChangeSource::ChangeFile(unique_id) = &change.original_source {
                if pre_release {
                    None
                } else {
                    Some(Action::RemoveFile {
                        path: RelativePathBuf::from(CHANGESET_DIR).join(unique_id.to_file_name()),
                    })
                }
            } else {
                None
            }
        }));
        Ok(actions)
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
    #[error("Found inconsistent versions in package: {} had {} and {} had {}", .0.path(), .0.version(), .1.path(), .1.version())]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code = "knope_versioning::inconsistent_versions",
            url = "https://knope.tech/reference/concepts/package/#version",
            help = "All files in a package must have the same version"
        )
    )]
    InconsistentVersions(Box<VersionedFile>, Box<VersionedFile>),
    #[error("Packages must have at least one versioned file")]
    NoPackages,
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
    #[error("Package version has already been updated")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code = "knope_versioning::package_already_bumped",
            url = "https://knope.tech/reference/concepts/package/#version",
            help = "You can only run a single BumpVersion or PrepareRelease step per workflow"
        )
    )]
    PackageAlreadyBumped,
}
