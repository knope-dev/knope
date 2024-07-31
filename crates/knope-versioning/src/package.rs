use std::{
    borrow::{Borrow, Cow},
    fmt,
    fmt::Display,
    ops::Deref,
};

use changesets::Release;
use itertools::Itertools;
#[cfg(feature = "miette")]
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    action::Action,
    changelog::Sections as ChangelogSections,
    changes::{
        conventional_commit::changes_from_commit_messages, Change, ChangeSource, CHANGESET_DIR,
    },
    versioned_file::{GoVersioning, SetError, VersionedFile},
    Version,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Package {
    pub name: Name,
    versioned_files: Vec<VersionedFile>,
    pub changelog_sections: ChangelogSections,
    pub scopes: Option<Vec<String>>,
}

impl Package {
    /// Try and combine a bunch of versioned files into one logical package.
    ///
    /// # Errors
    ///
    /// There must be at least one versioned file, and all files must have the same version.
    pub fn new(
        name: Name,
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
        Ok(Self {
            name,
            versioned_files,
            changelog_sections,
            scopes,
        })
    }

    #[must_use]
    pub fn versioned_files(&self) -> &[VersionedFile] {
        &self.versioned_files
    }

    #[must_use]
    pub fn get_version(&self) -> Option<&Version> {
        self.versioned_files.first().map(VersionedFile::version)
    }

    /// Returns the actions that must be taken to set this package to the new version.
    ///
    /// # Errors
    ///
    /// If the file is a `go.mod`, there are rules about what versions are allowed.
    ///
    /// If serialization of some sort fails, which is a bug, then this will return an error.
    pub fn set_version(
        self,
        new_version: &Version,
        go_versioning: GoVersioning,
    ) -> Result<Vec<Action>, SetError> {
        self.versioned_files
            .into_iter()
            .map(|f| f.set_version(new_version, go_versioning))
            .process_results(|iter| iter.flatten().collect())
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

    #[must_use]
    pub fn apply_changes(&self, changes: &[Change]) -> Vec<Action> {
        changes
            .iter()
            .filter_map(|change| {
                if let ChangeSource::ChangeFile(unique_id) = &change.original_source {
                    Some(Action::RemoveFile {
                        path: RelativePathBuf::from(CHANGESET_DIR).join(unique_id.to_file_name()),
                    })
                } else {
                    None
                }
            })
            .collect()
    }
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
