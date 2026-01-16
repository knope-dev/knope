use std::ops::Not;

use knope_versioning::{ConfigError, VersionedFileConfig};
use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use toml::Spanned;

use crate::changelog_section::ChangelogSection;

/// Represents a single package in `knope.toml`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct Package {
    /// The files which define the current version of the package.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub versioned_files: Vec<Spanned<VersionedFile>>,
    /// The path to the `CHANGELOG.md` file (if any) to be updated when running [`Step::PrepareRelease`].
    pub changelog: Option<RelativePathBuf>,
    /// Optional scopes that can be used to filter commits when running [`Step::PrepareRelease`].
    pub scopes: Option<Vec<String>>,
    /// Extra sections that should be added to the changelog from custom footers in commit messages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_changelog_sections: Vec<ChangelogSection>,
    /// The assets, if any, to upload with each release
    pub assets: Option<Assets>,
    #[serde(default, skip_serializing_if = "<&bool>::not")]
    pub ignore_go_major_versioning: bool,
}

/// A type that deserializes from either a single regex string or an array of regex strings.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Regexes {
    Regex(String),
    List(Vec<String>),
}

impl From<Regexes> for Vec<String> {
    fn from(value: Regexes) -> Self {
        match value {
            Regexes::Regex(s) => vec![s],
            Regexes::List(v) => v,
        }
    }
}

impl From<Vec<String>> for Regexes {
    fn from(value: Vec<String>) -> Self {
        if value.len() == 1 {
            Self::Regex(value.into_iter().next().unwrap_or_default())
        } else {
            Self::List(value)
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum VersionedFile {
    Simple(RelativePathBuf),
    Dependency {
        path: RelativePathBuf,
        dependency: String,
    },
    Regex {
        path: RelativePathBuf,
        regex: Regexes,
    },
}

impl From<VersionedFileConfig> for VersionedFile {
    fn from(config: VersionedFileConfig) -> Self {
        let (path, dependency, regex) = (config.as_path(), config.dependency, config.regex);
        if let Some(patterns) = regex {
            Self::Regex {
                path,
                regex: patterns.into(),
            }
        } else if let Some(dependency) = dependency {
            Self::Dependency { path, dependency }
        } else {
            Self::Simple(path)
        }
    }
}

impl TryFrom<VersionedFile> for VersionedFileConfig {
    type Error = ConfigError;

    fn try_from(value: VersionedFile) -> Result<Self, Self::Error> {
        match value {
            VersionedFile::Simple(path) => VersionedFileConfig::new(path, None, None),
            VersionedFile::Dependency { path, dependency } => {
                VersionedFileConfig::new(path, Some(dependency), None)
            }
            VersionedFile::Regex { path, regex } => {
                VersionedFileConfig::new(path, None, Some(regex.into()))
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Assets {
    Glob(String),
    List(Vec<Asset>),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Asset {
    pub path: RelativePathBuf,
    pub name: Option<String>,
}

impl Asset {
    /// Get the name of the asset
    ///
    /// # Errors
    ///
    /// If there is no explicit name set and the path does not have a file name
    pub fn name(&self) -> Result<String, AssetNameError> {
        if let Some(name) = &self.name {
            Ok(name.clone())
        } else {
            self.path
                .file_name()
                .ok_or(AssetNameError {
                    path: self.path.clone(),
                })
                .map(String::from)
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("No asset name set, and name could not be determined from path {path}")]
pub struct AssetNameError {
    path: RelativePathBuf,
}
