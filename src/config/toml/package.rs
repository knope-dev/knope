use std::{
    collections::{HashMap, HashSet},
    fmt,
    fmt::Display,
    path::PathBuf,
};

use git_conventional::FooterToken;
use indexmap::IndexMap;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::releases::{
    changelog, changelog::Changelog, versioned_file, versioned_file::VersionedFile,
    ChangelogSectionSource, PackageName,
};

/// Represents a single package in `knope.toml`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct Package {
    /// The files which define the current version of the package.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) versioned_files: Vec<PathBuf>,
    /// The path to the `CHANGELOG.md` file (if any) to be updated when running [`Step::PrepareRelease`].
    pub(crate) changelog: Option<PathBuf>,
    /// Optional scopes that can be used to filter commits when running [`Step::PrepareRelease`].
    pub(crate) scopes: Option<Vec<String>>,
    /// Extra sections that should be added to the changelog from custom footers in commit messages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) extra_changelog_sections: Vec<ChangelogSection>,
}

impl TryFrom<(Option<PackageName>, Package)> for crate::releases::Package {
    type Error = Error;

    fn try_from((name, package): (Option<PackageName>, Package)) -> Result<Self> {
        Ok(Self {
            versioned_files: package
                .versioned_files
                .into_iter()
                .map(VersionedFile::try_from)
                .collect::<std::result::Result<Vec<_>, _>>()?,
            name,
            changelog: package.changelog.map(Changelog::try_from).transpose()?,
            scopes: package.scopes,
            extra_changelog_sections: changelog_sections_toml_to_config(
                package.extra_changelog_sections,
            ),
            pending_changes: vec![],
            prepared_release: None,
            override_version: None,
        })
    }
}

impl TryFrom<(PackageName, Package)> for crate::releases::Package {
    type Error = Error;

    fn try_from((name, package): (PackageName, Package)) -> Result<Self> {
        Self::try_from((Some(name), package))
    }
}

impl TryFrom<Package> for crate::releases::Package {
    type Error = Error;

    fn try_from(package: Package) -> Result<Self> {
        Self::try_from((None, package))
    }
}

impl From<crate::releases::Package> for Package {
    fn from(package: crate::releases::Package) -> Self {
        Self {
            versioned_files: package
                .versioned_files
                .into_iter()
                .map(|file| file.path)
                .collect(),
            changelog: package.changelog.map(|changelog| changelog.path),
            scopes: package.scopes,
            extra_changelog_sections: changelog_sections_config_to_toml(
                package.extra_changelog_sections,
            ),
        }
    }
}

fn changelog_sections_toml_to_config(
    sections: Vec<ChangelogSection>,
) -> IndexMap<ChangelogSectionSource, ChangeLogSectionName> {
    let mut extra_changelog_sections = IndexMap::new();
    for section in sections {
        for footer in section.footers {
            extra_changelog_sections.insert(
                ChangelogSectionSource::CommitFooter(footer),
                section.name.clone(),
            );
        }
        for change_type in section.types {
            extra_changelog_sections.insert(change_type.into(), section.name.clone());
        }
    }
    let default_extra_footer = default_commit_footer();
    extra_changelog_sections
        .entry(default_extra_footer.into())
        .or_insert_with(|| ChangeLogSectionName::from("Notes"));
    extra_changelog_sections
}

fn default_commit_footer() -> CommitFooter {
    CommitFooter::from("Changelog-Note")
}

fn changelog_sections_config_to_toml(
    mut sections: IndexMap<ChangelogSectionSource, ChangeLogSectionName>,
) -> Vec<ChangelogSection> {
    let default_key: ChangelogSectionSource = default_commit_footer().into();
    sections.remove(&default_key);
    let mut footers = HashMap::new();
    let mut types = HashMap::new();
    let mut section_names = HashSet::new();
    for (source, name) in sections {
        section_names.insert(name.clone());
        match source {
            ChangelogSectionSource::CommitFooter(footer) => {
                footers.entry(name).or_insert_with(Vec::new).push(footer);
            }
            ChangelogSectionSource::CustomChangeType(change_type) => {
                types.entry(name).or_insert_with(Vec::new).push(change_type);
            }
        }
    }
    section_names
        .into_iter()
        .map(|name| ChangelogSection {
            footers: footers.remove(&name).unwrap_or_default(),
            types: types.remove(&name).unwrap_or_default(),
            name,
        })
        .collect()
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error(transparent)]
    Changelog(#[from] changelog::Error),
    #[error(transparent)]
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
