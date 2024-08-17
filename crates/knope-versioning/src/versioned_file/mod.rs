use std::{fmt::Debug, path::PathBuf};

use cargo::Cargo;
pub use go_mod::{GoMod, GoVersioning};
use package_json::PackageJson;
use pubspec::PubSpec;
use pyproject::PyProject;
use relative_path::RelativePathBuf;
use serde::{Serialize, Serializer};

use crate::{
    action::{
        ActionSet,
        ActionSet::{Single, Two},
    },
    semver::Version,
};

pub mod cargo;
mod go_mod;
mod package_json;
mod pubspec;
mod pyproject;

#[derive(Clone, Debug)]
pub enum VersionedFile {
    Cargo(Cargo),
    PubSpec(PubSpec),
    GoMod(GoMod),
    PackageJson(PackageJson),
    PyProject(PyProject),
}

impl VersionedFile {
    /// Create a new `VersionedFile`
    ///
    /// # Errors
    ///
    /// Depends on the format.
    /// If the content doesn't match the expected format, an error is returned.
    pub fn new<S: AsRef<str> + Debug>(
        path: Path,
        content: String,
        git_tags: &[S],
    ) -> Result<Self, Error> {
        match path.format {
            Format::Cargo => Cargo::new(path, &content)
                .map(VersionedFile::Cargo)
                .map_err(Error::Cargo),
            Format::PyProject => PyProject::new(path.as_path(), content)
                .map(VersionedFile::PyProject)
                .map_err(Error::PyProject),
            Format::PubSpec => PubSpec::new(path.as_path(), content)
                .map(VersionedFile::PubSpec)
                .map_err(Error::PubSpec),
            Format::GoMod => GoMod::new(path.as_path(), content, git_tags)
                .map(VersionedFile::GoMod)
                .map_err(Error::GoMod),
            Format::PackageJson => PackageJson::new(path.as_path(), content)
                .map(VersionedFile::PackageJson)
                .map_err(Error::PackageJson),
        }
    }

    #[must_use]
    pub fn path(&self) -> RelativePathBuf {
        match self {
            VersionedFile::Cargo(cargo) => cargo.get_path(),
            VersionedFile::PyProject(pyproject) => pyproject.get_path().clone(),
            VersionedFile::PubSpec(pubspec) => pubspec.get_path().clone(),
            VersionedFile::GoMod(gomod) => gomod.get_path().clone(),
            VersionedFile::PackageJson(package_json) => package_json.get_path().clone(),
        }
    }

    #[must_use]
    pub fn version(&self) -> &Version {
        match self {
            VersionedFile::Cargo(cargo) => cargo.get_version(),
            VersionedFile::PyProject(pyproject) => pyproject.get_version(),
            VersionedFile::PubSpec(pubspec) => pubspec.get_version(),
            VersionedFile::GoMod(gomod) => gomod.get_version(),
            VersionedFile::PackageJson(package_json) => package_json.get_version(),
        }
    }

    /// Set the version in the file.
    ///
    /// # Errors
    ///
    /// 1. If the file is `go.mod`, there are rules about what versions are allowed.
    pub(crate) fn set_version(
        self,
        new_version: &Version,
        go_versioning: GoVersioning,
    ) -> Result<ActionSet, SetError> {
        match self {
            VersionedFile::Cargo(cargo) => Ok(Single(cargo.set_version(new_version))),
            VersionedFile::PyProject(pyproject) => Ok(Single(pyproject.set_version(new_version))),
            VersionedFile::PubSpec(pubspec) => pubspec
                .set_version(new_version)
                .map_err(SetError::Yaml)
                .map(Single),
            VersionedFile::GoMod(gomod) => gomod
                .set_version(new_version, go_versioning)
                .map_err(SetError::GoMod)
                .map(Two),
            VersionedFile::PackageJson(package_json) => package_json
                .set_version(new_version)
                .map_err(SetError::Json)
                .map(Single),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum SetError {
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    GoMod(#[from] go_mod::SetError),
    #[error("Error serializing JSON, this is a bug: {0}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::versioned_file::json_serialize),
            help("This is a bug in knope, please report it."),
            url("https://github.com/knope-dev/knope/issues")
        )
    )]
    Json(#[from] serde_json::Error),
    #[error("Error serializing YAML, this is a bug: {0}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::versioned_file::yaml_serialize),
            help("This is a bug in knope, please report it."),
            url("https://github.com/knope-dev/knope/issues"),
        )
    )]
    Yaml(#[from] serde_yaml::Error),
}

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum Error {
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    Cargo(#[from] cargo::Error),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    PyProject(#[from] pyproject::Error),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    PubSpec(#[from] pubspec::Error),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    GoMod(#[from] go_mod::Error),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    PackageJson(#[from] package_json::Error),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Path {
    parent: Option<RelativePathBuf>,
    format: Format,
    pub dependency: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Format {
    Cargo,
    PyProject,
    PubSpec,
    GoMod,
    PackageJson,
}

impl Format {
    const fn file_name(self) -> &'static str {
        match self {
            Format::Cargo => "Cargo.toml",
            Format::PyProject => "pyproject.toml",
            Format::PubSpec => "pubspec.yaml",
            Format::GoMod => "go.mod",
            Format::PackageJson => "package.json",
        }
    }

    fn try_from(file_name: &str) -> Option<Self> {
        match file_name {
            "Cargo.toml" => Some(Format::Cargo),
            "pyproject.toml" => Some(Format::PyProject),
            "pubspec.yaml" => Some(Format::PubSpec),
            "go.mod" => Some(Format::GoMod),
            "package.json" => Some(Format::PackageJson),
            _ => None,
        }
    }

    const fn supports_dependencies(self) -> bool {
        matches!(self, Format::Cargo)
    }
}

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum FormatError {
    #[error("Unknown file: {0}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::versioned_file::unknown_file),
            help("Knope identities the type of file based on its name."),
            url("https://knope.tech/reference/config-file/packages#versioned_files")
        )
    )]
    UnknownFile(RelativePathBuf),
    #[error("Dependencies are not supported in {0} files")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::versioned_file::unsupported_dependency),
            help("Dependencies aren't supported in every file type."),
            url("https://knope.tech/reference/config-file/packages#versioned_files")
        )
    )]
    UnsupportedDependency(&'static str),
}

impl Path {
    /// Create a verified `Path` from a `RelativePathBuf`.
    ///
    /// # Errors
    ///
    /// If the file name does not match a supported format
    pub fn new(path: RelativePathBuf, dependency: Option<String>) -> Result<Self, FormatError> {
        let Some(file_name) = path.file_name() else {
            return Err(FormatError::UnknownFile(path));
        };
        let parent = path.parent().map(RelativePathBuf::from);
        let format = Format::try_from(file_name).ok_or(FormatError::UnknownFile(path))?;
        if !format.supports_dependencies() && dependency.is_some() {
            return Err(FormatError::UnsupportedDependency(format.file_name()));
        }
        Ok(Path {
            parent,
            format,
            dependency,
        })
    }

    #[must_use]
    pub fn as_path(&self) -> RelativePathBuf {
        self.parent.as_ref().map_or_else(
            || RelativePathBuf::from(self.format.file_name()),
            |parent| parent.join(self.format.file_name()),
        )
    }

    #[must_use]
    pub fn to_pathbuf(&self) -> PathBuf {
        self.as_path().to_path("")
    }

    #[must_use]
    pub const fn defaults() -> [Self; 5] {
        [
            Path {
                format: Format::Cargo,
                parent: None,
                dependency: None,
            },
            Path {
                parent: None,
                format: Format::GoMod,
                dependency: None,
            },
            Path {
                parent: None,
                format: Format::PackageJson,
                dependency: None,
            },
            Path {
                parent: None,
                format: Format::PubSpec,
                dependency: None,
            },
            Path {
                parent: None,
                format: Format::PyProject,
                dependency: None,
            },
        ]
    }
}

impl Serialize for Path {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_path().serialize(serializer)
    }
}

impl From<&Path> for PathBuf {
    fn from(path: &Path) -> Self {
        path.as_path().to_path("")
    }
}
