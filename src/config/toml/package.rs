use std::{fmt, fmt::Display, ops::Not, path::PathBuf};

use git_conventional::FooterToken;
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::step::releases::{
    changelog, changelog::Changelog, go::GoVersioning, package::Asset, versioned_file,
    versioned_file::VersionedFile, PackageName,
};

/// Represents a single package in `knope.toml`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct Package {
    /// The files which define the current version of the package.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) versioned_files: Vec<RelativePathBuf>,
    /// The path to the `CHANGELOG.md` file (if any) to be updated when running [`Step::PrepareRelease`].
    pub(crate) changelog: Option<PathBuf>,
    /// Optional scopes that can be used to filter commits when running [`Step::PrepareRelease`].
    pub(crate) scopes: Option<Vec<String>>,
    /// Extra sections that should be added to the changelog from custom footers in commit messages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) extra_changelog_sections: Vec<ChangelogSection>,
    assets: Option<Vec<Asset>>,
    #[serde(default, skip_serializing_if = "<&bool>::not")]
    ignore_go_major_versioning: bool,
}

impl TryFrom<(Option<PackageName>, Package)> for crate::step::releases::Package {
    type Error = Error;

    fn try_from((name, package): (Option<PackageName>, Package)) -> Result<Self> {
        Ok(Self {
            versioned_files: package
                .versioned_files
                .into_iter()
                .map(|rel| rel.to_path(""))
                .map(VersionedFile::try_from)
                .collect::<std::result::Result<Vec<_>, _>>()?,
            name,
            changelog: package.changelog.map(Changelog::try_from).transpose()?,
            scopes: package.scopes,
            changelog_sections: package.extra_changelog_sections.into(),
            pending_changes: vec![],
            prepared_release: None,
            override_version: None,
            assets: package.assets,
            go_versioning: if package.ignore_go_major_versioning {
                GoVersioning::IgnoreMajorRules
            } else {
                GoVersioning::default()
            },
        })
    }
}

impl TryFrom<(PackageName, Package)> for crate::step::releases::Package {
    type Error = Error;

    fn try_from((name, package): (PackageName, Package)) -> Result<Self> {
        Self::try_from((Some(name), package))
    }
}

impl TryFrom<Package> for crate::step::releases::Package {
    type Error = Error;

    fn try_from(package: Package) -> Result<Self> {
        Self::try_from((None, package))
    }
}

impl From<crate::step::releases::Package> for Package {
    fn from(package: crate::step::releases::Package) -> Self {
        Self {
            versioned_files: package
                .versioned_files
                .into_iter()
                .filter_map(|file| RelativePathBuf::from_path(file.path).ok())
                .collect(),
            changelog: package.changelog.map(|changelog| changelog.path),
            scopes: package.scopes,
            extra_changelog_sections: package.changelog_sections.into(),
            assets: package.assets,
            ignore_go_major_versioning: package.go_versioning == GoVersioning::IgnoreMajorRules,
        }
    }
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Changelog(#[from] changelog::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    VersionedFile(#[from] versioned_file::Error),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ChangelogSection {
    pub(crate) name: ChangeLogSectionName,
    #[serde(default)]
    pub(crate) footers: Vec<CommitFooter>,
    #[serde(default)]
    pub(crate) types: Vec<CustomChangeType>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct CommitFooter(String);

impl Display for CommitFooter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<FooterToken<'_>> for CommitFooter {
    fn from(token: FooterToken<'_>) -> Self {
        Self(token.to_string())
    }
}

impl From<&str> for CommitFooter {
    fn from(token: &str) -> Self {
        Self(token.into())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct CustomChangeType(String);

impl Display for CustomChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for CustomChangeType {
    fn from(token: String) -> Self {
        Self(token)
    }
}

impl From<&str> for CustomChangeType {
    fn from(token: &str) -> Self {
        Self(token.into())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct ChangeLogSectionName(String);

impl Display for ChangeLogSectionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ChangeLogSectionName {
    fn from(token: &str) -> Self {
        Self(token.into())
    }
}

impl AsRef<str> for ChangeLogSectionName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
