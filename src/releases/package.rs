use std::ffi::OsStr;
use std::fs::{read_to_string, write};
use std::path::{Path, PathBuf};

use itertools::Itertools;
use log::trace;
use semver::Version;

use crate::config::Package as PackageConfig;
use crate::releases::{cargo, get_current_versions_from_tag, go, package_json, pyproject};
use crate::step::StepError;
use crate::step::StepError::InvalidCargoToml;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Package {
    pub(crate) versioned_files: Vec<VersionedFile>,
    pub(crate) changelog: Option<Changelog>,
    pub(crate) name: Option<String>,
    pub(crate) scopes: Option<Vec<String>>,
}

impl Package {
    pub(crate) fn new(config: PackageConfig, name: Option<String>) -> Result<Self, StepError> {
        let versioned_files = config
            .versioned_files
            .into_iter()
            .map(VersionedFile::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        let changelog = config.changelog.map(Changelog::try_from).transpose()?;
        Ok(Package {
            versioned_files,
            changelog,
            name,
            scopes: config.scopes,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VersionedFile {
    /// The type of file format that `content` is.
    pub(crate) format: PackageFormat,
    /// The path to the file that was parsed.
    pub(crate) path: PathBuf,
    /// The raw content of the package manager file so it doesn't have to be read again.
    content: String,
}

impl TryFrom<PathBuf> for VersionedFile {
    type Error = StepError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let format = PackageFormat::try_from(&path)?;
        if !path.exists() {
            return Err(StepError::FileNotFound(path));
        }
        let content = read_to_string(&path)?;
        Ok(Self {
            format,
            path,
            content,
        })
    }
}

impl VersionedFile {
    pub(crate) fn get_version(&self, package_name: Option<&str>) -> Result<String, StepError> {
        self.format
            .get_version(&self.content, package_name, &self.path)
    }

    pub(crate) fn set_version(&mut self, version_str: &Version) -> Result<(), StepError> {
        self.content = self
            .format
            .set_version(self.content.clone(), version_str, &self.path)?;
        trace!("Writing {} to {}", self.content, self.path.display());
        write(&self.path, &self.content)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Changelog {
    pub(crate) path: PathBuf,
    pub(crate) content: String,
}

impl TryFrom<PathBuf> for Changelog {
    type Error = StepError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let content = if path.exists() {
            read_to_string(&path)?
        } else {
            String::new()
        };
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
    /// Get the version from `content` for package named `name` (if any name).
    /// `path` is used for error reporting.
    pub(crate) fn get_version(
        self,
        content: &str,
        name: Option<&str>,
        path: &Path,
    ) -> Result<String, StepError> {
        match self {
            PackageFormat::Cargo => {
                cargo::get_version(content).map_err(|_| InvalidCargoToml(path.into()))
            }
            PackageFormat::Poetry => pyproject::get_version(content, path)
                .map_err(|_| StepError::InvalidPyProject(path.into())),
            PackageFormat::JavaScript => package_json::get_version(content)
                .map_err(|_| StepError::InvalidPackageJson(path.into())),
            PackageFormat::Go => get_current_versions_from_tag(name).map(|current_versions| {
                current_versions
                    .unwrap_or_default()
                    .into_latest()
                    .to_string()
            }),
        }
    }

    /// Consume the `content` and return a version of it which contains `new_version`.
    ///
    /// `path` is only used for error reporting.
    pub(crate) fn set_version(
        self,
        content: String,
        new_version: &Version,
        path: &Path,
    ) -> Result<String, StepError> {
        match self {
            PackageFormat::Cargo => cargo::set_version(content, &new_version.to_string())
                .map_err(|_| InvalidCargoToml(path.into())),
            PackageFormat::Poetry => {
                pyproject::set_version(content, &new_version.to_string(), path)
                    .map_err(|_| StepError::InvalidPyProject(path.into()))
            }
            PackageFormat::JavaScript => {
                package_json::set_version(&content, &new_version.to_string())
                    .map_err(|_| StepError::InvalidPackageJson(path.into()))
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

/// Find all supported package formats in the current directory.
pub(crate) fn find_packages() -> Option<PackageConfig> {
    let default = PathBuf::from("CHANGELOG.md");
    let changelog = if Path::exists(&default) {
        Some(default)
    } else {
        None
    };

    let versioned_files = PACKAGE_FORMAT_FILE_NAMES
        .iter()
        .filter_map(|name| {
            let path = PathBuf::from(name);
            if path.exists() {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if versioned_files.is_empty() {
        return None;
    }
    Some(PackageConfig {
        versioned_files,
        changelog,
        scopes: None,
    })
}

/// Includes some helper text for the user to understand how to use the config to define packages.
pub(crate) fn suggested_package_toml() -> String {
    let package = find_packages();
    if let Some(package) = package {
        format!(
            "Found the package metadata files {files} in the current directory. You may need to add this \
            to your knope.toml:\n\n```\n[package]\n{toml}```",
            files = package.versioned_files.iter().map(|path| path.to_str().unwrap())
                .collect::<Vec<_>>()
                .join(", "),
            toml = toml::to_string(&package).unwrap()
        )
    } else {
        format!(
            "No supported package managers found in current directory. \
            The supported formats are {formats}. Here's how you might define a package for `Cargo.toml`:\
            \n\n```\n[package]\nversioned_files = [\"Cargo.toml\"]\nchangelog = \"CHANGELOG.md\"\n```",
            formats = PACKAGE_FORMAT_FILE_NAMES.join(", ")
        )
    }
}
