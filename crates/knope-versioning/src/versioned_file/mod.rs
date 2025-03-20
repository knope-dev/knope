use std::{fmt::Debug, path::PathBuf};

use cargo::Cargo;
pub use go_mod::{GoMod, GoVersioning};
use maven_pom::MavenPom;
use package_json::PackageJson;
use pubspec::PubSpec;
use pyproject::PyProject;
use relative_path::RelativePathBuf;
use serde::{Serialize, Serializer};
use tauri_conf_json::TauriConfJson;

use crate::{
    Action,
    action::ActionSet::{Single, Two},
    semver::Version,
    versioned_file::{cargo_lock::CargoLock, gleam::Gleam},
};

pub mod cargo;
mod cargo_lock;
mod gleam;
mod go_mod;
mod maven_pom;
mod package_json;
mod pubspec;
mod pyproject;
mod tauri_conf_json;

#[derive(Clone, Debug)]
pub enum VersionedFile {
    Cargo(Cargo),
    CargoLock(CargoLock),
    PubSpec(PubSpec),
    Gleam(Gleam),
    GoMod(GoMod),
    PackageJson(PackageJson),
    PyProject(PyProject),
    MavenPom(MavenPom),
    TauriConf(TauriConfJson),
    TauriMacosConf(TauriConfJson),
    TauriWindowsConf(TauriConfJson),
    TauriLinuxConf(TauriConfJson),
}

impl VersionedFile {
    /// Create a new `VersionedFile`
    ///
    /// # Errors
    ///
    /// Depends on the format.
    /// If the content doesn't match the expected format, an error is returned.
    pub fn new<S: AsRef<str> + Debug>(
        config: &Config,
        content: String,
        git_tags: &[S],
    ) -> Result<Self, Error> {
        match config.format {
            Format::Cargo => Cargo::new(config.as_path(), &content)
                .map(VersionedFile::Cargo)
                .map_err(Error::Cargo),
            Format::CargoLock => CargoLock::new(config.as_path(), &content)
                .map(VersionedFile::CargoLock)
                .map_err(Error::CargoLock),
            Format::PyProject => PyProject::new(config.as_path(), content)
                .map(VersionedFile::PyProject)
                .map_err(Error::PyProject),
            Format::PubSpec => PubSpec::new(config.as_path(), content)
                .map(VersionedFile::PubSpec)
                .map_err(Error::PubSpec),
            Format::Gleam => Gleam::new(config.as_path(), &content)
                .map(VersionedFile::Gleam)
                .map_err(Error::Gleam),
            Format::GoMod => GoMod::new(config.as_path(), content, git_tags)
                .map(VersionedFile::GoMod)
                .map_err(Error::GoMod),
            Format::PackageJson => PackageJson::new(config.as_path(), content)
                .map(VersionedFile::PackageJson)
                .map_err(Error::PackageJson),
            Format::MavenPom => MavenPom::new(config.as_path(), content)
                .map(VersionedFile::MavenPom)
                .map_err(Error::MavenPom),
            Format::TauriConf => TauriConfJson::new(config.as_path(), content)
                .map(VersionedFile::TauriConf)
                .map_err(Error::TauriConfJson),
            Format::TauriMacosConf => TauriConfJson::new(config.as_path(), content)
                .map(VersionedFile::TauriMacosConf)
                .map_err(Error::TauriConfJson),
            Format::TauriWindowsConf => TauriConfJson::new(config.as_path(), content)
                .map(VersionedFile::TauriWindowsConf)
                .map_err(Error::TauriConfJson),
            Format::TauriLinuxConf => TauriConfJson::new(config.as_path(), content)
                .map(VersionedFile::TauriLinuxConf)
                .map_err(Error::TauriConfJson),
        }
    }

    #[must_use]
    pub fn path(&self) -> &RelativePathBuf {
        match self {
            VersionedFile::Cargo(cargo) => &cargo.path,
            VersionedFile::CargoLock(cargo_lock) => &cargo_lock.path,
            VersionedFile::PyProject(pyproject) => &pyproject.path,
            VersionedFile::PubSpec(pubspec) => pubspec.get_path(),
            VersionedFile::Gleam(gleam) => &gleam.path,
            VersionedFile::GoMod(gomod) => gomod.get_path(),
            VersionedFile::PackageJson(package_json) => package_json.get_path(),
            VersionedFile::MavenPom(maven_pom) => &maven_pom.path,
            VersionedFile::TauriConf(tauri_conf) => &tauri_conf.get_path(),
            VersionedFile::TauriMacosConf(tauri_conf) => &tauri_conf.get_path(),
            VersionedFile::TauriWindowsConf(tauri_conf) => &tauri_conf.get_path(),
            VersionedFile::TauriLinuxConf(tauri_conf) => &tauri_conf.get_path(),
        }
    }

    /// Get the package version from the file.
    ///
    /// # Errors
    ///
    /// If there's no package version for this type of file (e.g., lock file, dependency file).
    pub fn version(&self) -> Result<Version, Error> {
        match self {
            VersionedFile::Cargo(cargo) => cargo.get_version().map_err(Error::Cargo),
            VersionedFile::CargoLock(_) => Err(Error::NoVersion),
            VersionedFile::PyProject(pyproject) => Ok(pyproject.version.clone()),
            VersionedFile::PubSpec(pubspec) => Ok(pubspec.get_version().clone()),
            VersionedFile::Gleam(gleam) => Ok(gleam.get_version().map_err(Error::Gleam)?),
            VersionedFile::GoMod(gomod) => Ok(gomod.get_version().clone()),
            VersionedFile::PackageJson(package_json) => Ok(package_json.get_version().clone()),
            VersionedFile::MavenPom(maven_pom) => maven_pom.get_version().map_err(Error::MavenPom),
            VersionedFile::TauriConf(tauri_conf) => Ok(tauri_conf.get_version().clone()),
            VersionedFile::TauriMacosConf(tauri_conf) => Ok(tauri_conf.get_version().clone()),
            VersionedFile::TauriWindowsConf(tauri_conf) => Ok(tauri_conf.get_version().clone()),
            VersionedFile::TauriLinuxConf(tauri_conf) => Ok(tauri_conf.get_version().clone()),
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
        dependency: Option<&str>,
        go_versioning: GoVersioning,
    ) -> Result<Self, SetError> {
        match self {
            Self::Cargo(cargo) => Ok(Self::Cargo(cargo.set_version(new_version, dependency))),
            Self::CargoLock(cargo_lock) => cargo_lock
                .set_version(new_version, dependency)
                .map(Self::CargoLock)
                .map_err(SetError::CargoLock),
            Self::PyProject(pyproject) => Ok(Self::PyProject(pyproject.set_version(new_version))),
            Self::PubSpec(pubspec) => pubspec
                .set_version(new_version)
                .map_err(SetError::Yaml)
                .map(Self::PubSpec),
            Self::Gleam(gleam) => Ok(Self::Gleam(gleam.set_version(new_version))),
            Self::GoMod(gomod) => gomod
                .set_version(new_version.clone(), go_versioning)
                .map_err(SetError::GoMod)
                .map(Self::GoMod),
            Self::PackageJson(package_json) => package_json
                .set_version(new_version)
                .map_err(SetError::Json)
                .map(Self::PackageJson),
            Self::MavenPom(maven_pom) => maven_pom
                .set_version(new_version)
                .map_err(SetError::MavenPom)
                .map(Self::MavenPom),
            Self::TauriConf(tauri_conf) => tauri_conf
                .set_version(new_version)
                .map_err(SetError::Json)
                .map(Self::TauriConf),
            Self::TauriMacosConf(tauri_conf) => tauri_conf
                .set_version(new_version)
                .map_err(SetError::Json)
                .map(Self::TauriMacosConf),
            Self::TauriWindowsConf(tauri_conf) => tauri_conf
                .set_version(new_version)
                .map_err(SetError::Json)
                .map(Self::TauriWindowsConf),
            Self::TauriLinuxConf(tauri_conf) => tauri_conf
                .set_version(new_version)
                .map_err(SetError::Json)
                .map(Self::TauriLinuxConf),
        }
    }

    pub fn write(self) -> Option<impl IntoIterator<Item = Action>> {
        match self {
            Self::Cargo(cargo) => cargo.write().map(Single),
            Self::CargoLock(cargo_lock) => cargo_lock.write().map(Single),
            Self::PyProject(pyproject) => pyproject.write().map(Single),
            Self::PubSpec(pubspec) => pubspec.write().map(Single),
            Self::Gleam(gleam) => gleam.write().map(Single),
            Self::GoMod(gomod) => gomod.write().map(Two),
            Self::PackageJson(package_json) => package_json.write().map(Single),
            Self::MavenPom(maven_pom) => maven_pom.write().map(Single),
            Self::TauriConf(tauri_conf) => tauri_conf.write().map(Single),
            Self::TauriMacosConf(tauri_conf) => tauri_conf.write().map(Single),
            Self::TauriWindowsConf(tauri_conf) => tauri_conf.write().map(Single),
            Self::TauriLinuxConf(tauri_conf) => tauri_conf.write().map(Single),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum SetError {
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
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    GoMod(#[from] go_mod::SetError),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    CargoLock(#[from] cargo_lock::SetError),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    MavenPom(#[from] maven_pom::Error),
}

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum Error {
    #[error("This file can't contain a version")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::versioned_file::no_version),
            help("This is likely a bug, please report it."),
            url("https://github.com/knope-dev/knope/issues")
        )
    )]
    NoVersion,
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    Cargo(#[from] cargo::Error),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    CargoLock(#[from] cargo_lock::Error),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    PyProject(#[from] pyproject::Error),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    PubSpec(#[from] pubspec::Error),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    Gleam(#[from] gleam::Error),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    GoMod(#[from] go_mod::Error),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    PackageJson(#[from] package_json::Error),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    MavenPom(#[from] maven_pom::Error),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    TauriConfJson(#[from] tauri_conf_json::Error),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Format {
    Cargo,
    CargoLock,
    PyProject,
    PubSpec,
    Gleam,
    GoMod,
    PackageJson,
    MavenPom,
    TauriConf,
    TauriMacosConf,
    TauriWindowsConf,
    TauriLinuxConf,
}

impl Format {
    pub(crate) const fn file_name(self) -> &'static str {
        match self {
            Format::Cargo => "Cargo.toml",
            Format::CargoLock => "Cargo.lock",
            Format::PyProject => "pyproject.toml",
            Format::PubSpec => "pubspec.yaml",
            Format::Gleam => "gleam.toml",
            Format::GoMod => "go.mod",
            Format::PackageJson => "package.json",
            Format::MavenPom => "pom.xml",
            Format::TauriConf => "tauri.conf.json",
            Format::TauriMacosConf => "tauri.macos.conf.json",
            Format::TauriWindowsConf => "tauri.windows.conf.json",
            Format::TauriLinuxConf => "tauri.linux.conf.json",
        }
    }

    fn try_from(file_name: &str) -> Option<Self> {
        match file_name {
            "Cargo.toml" => Some(Format::Cargo),
            "Cargo.lock" => Some(Format::CargoLock),
            "pyproject.toml" => Some(Format::PyProject),
            "pubspec.yaml" => Some(Format::PubSpec),
            "gleam.toml" => Some(Format::Gleam),
            "go.mod" => Some(Format::GoMod),
            "package.json" => Some(Format::PackageJson),
            "pom.xml" => Some(Format::MavenPom),
            "tauri.conf.json" => Some(Format::TauriConf),
            "tauri.macos.conf.json" => Some(Format::TauriMacosConf),
            "tauri.windows.conf.json" => Some(Format::TauriWindowsConf),
            "tauri.linux.conf.json" => Some(Format::TauriLinuxConf),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
#[error("Unknown file: {path}")]
#[cfg_attr(
    feature = "miette",
    diagnostic(
        code(knope_versioning::versioned_file::unknown_file),
        help("Knope identities the type of file based on its name."),
        url("https://knope.tech/reference/config-file/packages#versioned_files")
    )
)]
pub struct UnknownFile {
    pub path: RelativePathBuf,
}

/// The configuration of a versioned file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    /// The directory that the file is in
    parent: Option<RelativePathBuf>,
    /// The type of file
    pub(crate) format: Format,
    /// If, within the file, we're versioning a dependency (not the entire package)
    pub dependency: Option<String>,
}

impl Config {
    /// Create a verified `Config` from a `RelativePathBuf`.
    ///
    /// # Errors
    ///
    /// If the file name does not match a supported format
    pub fn new(path: RelativePathBuf, dependency: Option<String>) -> Result<Self, UnknownFile> {
        let Some(file_name) = path.file_name() else {
            return Err(UnknownFile { path });
        };
        let parent = path.parent().map(RelativePathBuf::from);
        let format = Format::try_from(file_name).ok_or(UnknownFile { path })?;
        Ok(Config {
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
    pub const fn defaults() -> [Self; 6] {
        [
            Config {
                format: Format::Cargo,
                parent: None,
                dependency: None,
            },
            Config {
                parent: None,
                format: Format::GoMod,
                dependency: None,
            },
            Config {
                parent: None,
                format: Format::PackageJson,
                dependency: None,
            },
            Config {
                parent: None,
                format: Format::PubSpec,
                dependency: None,
            },
            Config {
                parent: None,
                format: Format::PyProject,
                dependency: None,
            },
            Config {
                parent: None,
                format: Format::MavenPom,
                dependency: None,
            },
        ]
    }
}

impl Serialize for Config {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_path().serialize(serializer)
    }
}

impl From<&Config> for PathBuf {
    fn from(path: &Config) -> Self {
        path.as_path().to_path("")
    }
}

impl PartialEq<RelativePathBuf> for Config {
    fn eq(&self, other: &RelativePathBuf) -> bool {
        let other_parent = other.parent();
        let parent = self.parent.as_deref();

        let parents_match = match (parent, other_parent) {
            (Some(parent), Some(other_parent)) => parent == other_parent,
            (None, None) => true,
            (Some(parent), None) if parent == "" => true,
            (None, Some(other_parent)) if other_parent == "" => true,
            _ => false,
        };

        parents_match
            && other
                .file_name()
                .is_some_and(|file_name| file_name == self.format.file_name())
    }
}

impl PartialEq<Config> for RelativePathBuf {
    fn eq(&self, other: &Config) -> bool {
        other == self
    }
}
