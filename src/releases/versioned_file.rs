use std::{
    ffi::OsStr,
    fs::{read_to_string, write},
    path::{Path, PathBuf},
};

use itertools::Itertools;
use log::trace;
use miette::Diagnostic;
use thiserror::Error;

use crate::releases::{
    cargo, get_current_versions_from_tag, go, package_json, pyproject, semver::Version,
};

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
    type Error = Error;

    fn try_from(path: PathBuf) -> Result<Self> {
        let format = PackageFormat::try_from(&path)?;
        let content = read_to_string(&path).map_err(|e| Error::Io(path.clone(), e))?;
        Ok(Self {
            format,
            path,
            content,
        })
    }
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error("Error reading file {0}: {1}")]
    #[diagnostic(
        code(versioned_file::io),
        help("Please check that the file exists and is readable.")
    )]
    Io(PathBuf, #[source] std::io::Error),
    #[error("Path is not a file: {0}")]
    #[diagnostic(
        code(versioned_file::not_a_file),
        help("A versioned file must be a valid relative path to a file.")
    )]
    NotAFile(PathBuf),
    #[error("The versioned file {0} is not a supported format")]
    #[diagnostic(
        code(step::versioned_file_format),
        help("All filed included in [[packages]] versioned_files must be a supported format"),
        url("https://knope-dev.github.io/knope/config/packages.html#supported-formats-for-versioning")
    )]
    VersionedFileFormat(PathBuf),
    #[error("The file {0} was an incorrect format")]
    #[diagnostic(
        code(step::invalid_cargo_toml),
        help("knope expects the Cargo.toml file to have a `package.version` property. Workspace support is coming soon!"),
        url("https://knope-dev.github.io/knope/config/packages.html#supported-formats-for-versioning")
    )]
    InvalidCargoToml(PathBuf),
    #[error("The file {0} was an incorrect format")]
    #[diagnostic(
        code(step::invalid_package_json),
        help("knope expects the package.json file to be an object with a top level `version` property"),
        url("https://knope-dev.github.io/knope/config/packages.html#supported-formats-for-versioning")
    )]
    InvalidPackageJson(PathBuf),
    #[error("The file {0} was an incorrect format")]
    #[diagnostic(
        code(step::invalid_pyproject),
        help(
        "knope expects the pyproject.toml file to have a `tool.poetry.version` or \
                `project.version` property. If you use a different location for your version, please \
                open an issue to add support."
        ),
        url("https://knope-dev.github.io/knope/config/packages.html#supported-formats-for-versioning")
    )]
    InvalidPyProject(PathBuf),
    #[error(transparent)]
    Git(#[from] crate::releases::git::Error),
    #[error(transparent)]
    Go(#[from] crate::releases::go::Error),
}

type Result<T> = std::result::Result<T, Error>;

impl VersionedFile {
    pub(crate) fn get_version(&self, package_name: Option<&str>) -> Result<String> {
        self.format
            .get_version(&self.content, package_name, &self.path)
    }

    pub(crate) fn set_version(&mut self, version_str: &Version) -> Result<()> {
        self.content = self
            .format
            .set_version(self.content.clone(), version_str, &self.path)?;
        trace!("Writing {} to {}", self.content, self.path.display());
        write(&self.path, &self.content).map_err(|e| Error::Io(self.path.clone(), e))?;
        Ok(())
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
    type Error = Error;

    fn try_from(path: &PathBuf) -> Result<Self> {
        let file_name = path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| Error::NotAFile(path.clone()))?;
        PACKAGE_FORMAT_FILE_NAMES
            .iter()
            .find_position(|&name| *name == file_name)
            .and_then(|(pos, _)| ALL_PACKAGE_FORMATS.get(pos).copied())
            .ok_or_else(|| Error::VersionedFileFormat(path.clone()))
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
    ) -> Result<String> {
        match self {
            PackageFormat::Cargo => {
                cargo::get_version(content).map_err(|_| Error::InvalidCargoToml(path.into()))
            }
            PackageFormat::Poetry => pyproject::get_version(content, path)
                .map_err(|_| Error::InvalidPyProject(path.into())),
            PackageFormat::JavaScript => package_json::get_version(content)
                .map_err(|_| Error::InvalidPackageJson(path.into())),
            PackageFormat::Go => get_current_versions_from_tag(name)
                .map(|current_versions| {
                    current_versions
                        .into_latest()
                        .unwrap_or_default()
                        .to_string()
                })
                .map_err(Error::from),
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
    ) -> Result<String> {
        match self {
            PackageFormat::Cargo => cargo::set_version(content, &new_version.to_string())
                .map_err(|_| Error::InvalidCargoToml(path.into())),
            PackageFormat::Poetry => {
                pyproject::set_version(content, &new_version.to_string(), path)
                    .map_err(|_| Error::InvalidPyProject(path.into()))
            }
            PackageFormat::JavaScript => {
                package_json::set_version(&content, &new_version.to_string())
                    .map_err(|_| Error::InvalidPackageJson(path.into()))
            }
            PackageFormat::Go => go::set_version(content, new_version).map_err(Error::from),
        }
    }
}

const ALL_PACKAGE_FORMATS: [PackageFormat; 4] = [
    PackageFormat::Cargo,
    PackageFormat::Go,
    PackageFormat::JavaScript,
    PackageFormat::Poetry,
];
pub(crate) const PACKAGE_FORMAT_FILE_NAMES: [&str; ALL_PACKAGE_FORMATS.len()] =
    ["Cargo.toml", "go.mod", "package.json", "pyproject.toml"];
