use std::ffi::OsStr;
use std::fs::{read_to_string, write};
use std::path::{Path, PathBuf};

use itertools::Itertools;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::releases::{cargo, get_current_versions_from_tag, go, package_json, pyproject};
use crate::step::StepError;

/// Represents an entry in the `[[packages]]` section of `knope.toml`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct PackageConfig {
    /// The files which define the current version of the package.
    pub(crate) versioned_files: Vec<PathBuf>,
    /// The path to the `CHANGELOG.md` file (if any) to be updated when running [`crate::Step::PrepareRelease`].
    pub(crate) changelog: Option<PathBuf>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Package {
    pub(crate) versioned_files: Vec<VersionedFile>,
    pub(crate) changelog: Option<Changelog>,
}

impl TryFrom<PackageConfig> for Package {
    type Error = StepError;

    fn try_from(config: PackageConfig) -> Result<Self, Self::Error> {
        let versioned_files = config
            .versioned_files
            .into_iter()
            .map(VersionedFile::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        let changelog = config.changelog.map(Changelog::try_from).transpose()?;
        Ok(Package {
            versioned_files,
            changelog,
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct VersionedFile {
    /// The type of file format that `content` is.
    pub(crate) format: PackageFormat,
    /// The path to the file that was parsed.
    path: PathBuf,
    /// The raw content of the package manager file so it doesn't have to be read again.
    content: String,
}

impl TryFrom<PathBuf> for VersionedFile {
    type Error = StepError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let format = PackageFormat::try_from(&path)?;
        let content = read_to_string(&path)?;
        Ok(Self {
            format,
            path,
            content,
        })
    }
}

impl VersionedFile {
    pub(crate) fn get_version(&self) -> Result<String, StepError> {
        self.format.get_version(&self.content)
    }

    pub(crate) fn set_version(self, version_str: &Version) -> Result<(), StepError> {
        let new_content = self.format.set_version(self.content, version_str)?;
        write(&self.path, new_content)?;
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Changelog {
    path: PathBuf,
    content: String,
}

impl TryFrom<PathBuf> for Changelog {
    type Error = StepError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let content = read_to_string(&path)?;
        Ok(Self { path, content })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PackageFormat {
    Cargo,
    Go,
    JavaScript,
    Poetry,
}

impl TryFrom<&PathBuf> for PackageFormat {
    type Error = StepError;

    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        let file_name = path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| StepError::FileNotFound(path.clone()))?;
        PACKAGE_FORMAT_FILE_NAMES
            .iter()
            .find_position(|&name| *name == file_name)
            .map(|(pos, _)| ALL_PACKAGE_FORMATS[pos])
            .ok_or_else(|| StepError::VersionedFileFormat(path.clone()))
    }
}

impl PackageFormat {
    pub(crate) fn get_version(self, content: &str) -> Result<String, StepError> {
        match self {
            PackageFormat::Cargo => cargo::get_version(content),
            PackageFormat::Poetry => pyproject::get_version(content),
            PackageFormat::JavaScript => package_json::get_version(content),
            PackageFormat::Go => get_current_versions_from_tag().map(|current_versions| {
                current_versions
                    .unwrap_or_default()
                    .into_latest()
                    .to_string()
            }),
        }
    }

    pub(crate) fn set_version(
        self,
        content: String,
        new_version: &Version,
    ) -> Result<String, StepError> {
        match self {
            PackageFormat::Cargo => cargo::set_version(content, &new_version.to_string()),
            PackageFormat::Poetry => pyproject::set_version(content, &new_version.to_string()),
            PackageFormat::JavaScript => {
                package_json::set_version(&content, &new_version.to_string())
            }
            PackageFormat::Go => go::set_version(content, new_version),
        }
    }
}

const ALL_PACKAGE_FORMATS: [PackageFormat; 4] = [
    PackageFormat::Cargo,
    PackageFormat::Go,
    PackageFormat::JavaScript,
    PackageFormat::Poetry,
];
const PACKAGE_FORMAT_FILE_NAMES: [&str; ALL_PACKAGE_FORMATS.len()] =
    ["Cargo.toml", "go.mod", "package.json", "pyproject.toml"];

/// Find the first supported package manager in the current directory that can be added to generated config.
pub(crate) fn find_packages() -> Vec<PackageConfig> {
    let default = PathBuf::from("CHANGELOG.md");
    let changelog = if Path::exists(&default) {
        Some(default)
    } else {
        None
    };

    for supported in PACKAGE_FORMAT_FILE_NAMES.map(PathBuf::from) {
        if Path::exists(&supported) {
            return vec![PackageConfig {
                versioned_files: vec![supported],
                changelog,
            }];
        }
    }
    return vec![];
}

/// Includes some helper text for the user to understand how to use the config to define packages.
pub(crate) fn suggested_package_toml() -> String {
    let packages = find_packages();
    if packages.is_empty() {
        return format!(
                "No supported package managers found in current directory. \
                The supported formats are {formats}. Here's how you might define a package for `Cargo.toml`:\
                \n\n```\n[[packages]]\nversioned_files = [\"Cargo.toml\"]\nchangelog = \"CHANGELOG.md\"\n```",
                formats = PACKAGE_FORMAT_FILE_NAMES.join(", ")
            );
    }
    return format!(
        "Found the package metadata file {file} in the current directory. You may need to add this \
        to your knope.toml:\n\n```\n[[packages]]\n{toml}```",
        file = packages[0].versioned_files[0].to_str().unwrap(),
        toml = toml::to_string(&packages[0]).unwrap()
    );
}
