use std::{fmt::Debug, path::PathBuf};

use relative_path::RelativePathBuf;
use serde::{Serialize, Serializer};

pub use self::go_mod::GoVersioning;
use self::{
    cargo::Cargo, cargo_lock::CargoLock, deno_json::DenoJson, deno_lock::DenoLock, gleam::Gleam,
    go_mod::GoMod, maven_pom::MavenPom, package_json::PackageJson,
    package_lock_json::PackageLockJson, pubspec::PubSpec, pyproject::PyProject,
    regex_file::RegexFile, tauri_conf_json::TauriConfJson,
};
use crate::{
    Action,
    action::ActionSet::{Single, Two},
    semver::Version,
};

pub mod cargo;
mod cargo_lock;
mod deno_json;
mod deno_lock;
mod gleam;
mod go_mod;
mod maven_pom;
mod package_json;
mod package_lock_json;
mod pubspec;
mod pyproject;
mod regex_file;
mod tauri_conf_json;

#[derive(Clone, Debug)]
pub enum VersionedFile {
    Cargo(Cargo),
    CargoLock(CargoLock),
    DenoJson(DenoJson),
    DenoLock(DenoLock),
    PubSpec(PubSpec),
    Gleam(Gleam),
    GoMod(GoMod),
    PackageJson(PackageJson),
    PackageLockJson(PackageLockJson),
    PyProject(PyProject),
    MavenPom(MavenPom),
    TauriConf(TauriConfJson),
    TauriMacosConf(TauriConfJson),
    TauriWindowsConf(TauriConfJson),
    TauriLinuxConf(TauriConfJson),
    RegexFile(RegexFile),
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
            Format::DenoJson => DenoJson::new(config.as_path(), content)
                .map(VersionedFile::DenoJson)
                .map_err(Error::DenoJson),
            Format::DenoLock => DenoLock::new(config.as_path(), &content)
                .map(VersionedFile::DenoLock)
                .map_err(Error::DenoLock),
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
            Format::PackageLockJson => PackageLockJson::new(config.as_path(), &content)
                .map(VersionedFile::PackageLockJson)
                .map_err(Error::PackageLockJson),
            Format::MavenPom => MavenPom::new(config.as_path(), content)
                .map(VersionedFile::MavenPom)
                .map_err(Error::MavenPom),
            Format::TauriConf => TauriConfJson::new(config.as_path(), content)
                .map(VersionedFile::TauriConf)
                .map_err(Error::TauriConfJson),
            Format::RegexFile => {
                let patterns = config
                    .regex
                    .as_ref()
                    .ok_or_else(|| {
                        Error::RegexFile(regex_file::Error::NoMatch {
                            regex: String::new(),
                            path: config.as_path(),
                        })
                    })?
                    .clone();
                RegexFile::new(config.as_path(), content, patterns)
                    .map(VersionedFile::RegexFile)
                    .map_err(Error::RegexFile)
            }
        }
    }

    #[must_use]
    pub fn path(&self) -> &RelativePathBuf {
        match self {
            VersionedFile::Cargo(cargo) => &cargo.path,
            VersionedFile::CargoLock(cargo_lock) => &cargo_lock.path,
            VersionedFile::DenoJson(deno_json) => deno_json.get_path(),
            VersionedFile::DenoLock(deno_lock) => deno_lock.get_path(),
            VersionedFile::PyProject(pyproject) => &pyproject.path,
            VersionedFile::PubSpec(pubspec) => pubspec.get_path(),
            VersionedFile::Gleam(gleam) => &gleam.path,
            VersionedFile::GoMod(gomod) => gomod.get_path(),
            VersionedFile::PackageJson(package_json) => package_json.get_path(),
            VersionedFile::PackageLockJson(package_lock_json) => package_lock_json.get_path(),
            VersionedFile::MavenPom(maven_pom) => &maven_pom.path,
            VersionedFile::TauriConf(tauri_conf)
            | VersionedFile::TauriMacosConf(tauri_conf)
            | VersionedFile::TauriWindowsConf(tauri_conf)
            | VersionedFile::TauriLinuxConf(tauri_conf) => tauri_conf.get_path(),
            VersionedFile::RegexFile(regex_file) => &regex_file.path,
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
            VersionedFile::CargoLock(_) | VersionedFile::DenoLock(_) => Err(Error::NoVersion),
            VersionedFile::DenoJson(deno_json) => deno_json.get_version().ok_or(Error::NoVersion),
            VersionedFile::PyProject(pyproject) => Ok(pyproject.version.clone()),
            VersionedFile::PubSpec(pubspec) => Ok(pubspec.get_version().clone()),
            VersionedFile::Gleam(gleam) => Ok(gleam.get_version().map_err(Error::Gleam)?),
            VersionedFile::GoMod(gomod) => Ok(gomod.get_version().clone()),
            VersionedFile::PackageJson(package_json) => Ok(package_json.get_version().clone()),
            VersionedFile::PackageLockJson(package_lock_json) => package_lock_json
                .get_version()
                .map_err(Error::PackageLockJson),
            VersionedFile::MavenPom(maven_pom) => maven_pom.get_version().map_err(Error::MavenPom),
            VersionedFile::TauriConf(tauri_conf)
            | VersionedFile::TauriMacosConf(tauri_conf)
            | VersionedFile::TauriWindowsConf(tauri_conf)
            | VersionedFile::TauriLinuxConf(tauri_conf) => Ok(tauri_conf.get_version().clone()),
            VersionedFile::RegexFile(regex_file) => {
                regex_file.get_version().map_err(Error::RegexFile)
            }
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
            Self::DenoJson(deno_json) => deno_json
                .set_version(new_version, dependency)
                .map_err(SetError::Json)
                .map(Self::DenoJson),
            Self::DenoLock(deno_lock) => deno_lock
                .set_version(new_version, dependency)
                .map_err(SetError::DenoLock)
                .map(Self::DenoLock),
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
                .set_version(new_version, dependency)
                .map_err(SetError::Json)
                .map(Self::PackageJson),
            Self::PackageLockJson(package_lock_json) => Ok(Self::PackageLockJson(
                package_lock_json.set_version(new_version, dependency),
            )),
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
            Self::RegexFile(regex_file) => Ok(Self::RegexFile(regex_file.set_version(new_version))),
        }
    }

    pub fn write(self) -> Option<impl IntoIterator<Item = Action>> {
        match self {
            Self::Cargo(cargo) => cargo.write().map(Single),
            Self::CargoLock(cargo_lock) => cargo_lock.write().map(Single),
            Self::DenoJson(deno_json) => deno_json.write().map(Single),
            Self::PyProject(pyproject) => pyproject.write().map(Single),
            Self::PubSpec(pubspec) => pubspec.write().map(Single),
            Self::Gleam(gleam) => gleam.write().map(Single),
            Self::GoMod(gomod) => gomod.write().map(Two),
            Self::PackageJson(package_json) => package_json.write().map(Single),
            Self::PackageLockJson(package_lock_json) => package_lock_json.write().map(Single),
            Self::DenoLock(deno_lock) => deno_lock.write().map(Single),
            Self::MavenPom(maven_pom) => maven_pom.write().map(Single),
            Self::TauriConf(tauri_conf)
            | Self::TauriMacosConf(tauri_conf)
            | Self::TauriWindowsConf(tauri_conf)
            | Self::TauriLinuxConf(tauri_conf) => tauri_conf.write().map(Single),
            Self::RegexFile(regex_file) => regex_file.write().map(Single),
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
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    DenoLock(#[from] deno_lock::Error),
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
    DenoJson(#[from] deno_json::Error),
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
    PackageLockJson(#[from] package_lock_json::Error),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    MavenPom(#[from] maven_pom::Error),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    TauriConfJson(#[from] tauri_conf_json::Error),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    DenoLock(#[from] deno_lock::Error),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    RegexFile(#[from] regex_file::Error),
}

/// All the file types supported for versioning.
///
/// Be sure to add new variants to [`Format::FILE_NAMES`]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Format {
    Cargo,
    CargoLock,
    DenoJson,
    PyProject,
    PubSpec,
    Gleam,
    GoMod,
    PackageJson,
    PackageLockJson,
    DenoLock,
    MavenPom,
    TauriConf,
    RegexFile,
}

impl Format {
    /// This is how Knope automatically detects a file type based on its name.
    const FILE_NAMES: &'static [(&'static str, Self)] = &[
        ("Cargo.toml", Format::Cargo),
        ("Cargo.lock", Format::CargoLock),
        ("deno.json", Format::DenoJson),
        ("deno.lock", Format::DenoLock),
        ("gleam.toml", Format::Gleam),
        ("go.mod", Format::GoMod),
        ("package.json", Format::PackageJson),
        ("package-lock.json", Format::PackageLockJson),
        ("pom.xml", Format::MavenPom),
        ("pubspec.yaml", Format::PubSpec),
        ("pyproject.toml", Format::PyProject),
        ("tauri.conf.json", Format::TauriConf),
        ("tauri.macos.conf.json", Format::TauriConf),
        ("tauri.windows.conf.json", Format::TauriConf),
        ("tauri.linux.conf.json", Format::TauriConf),
    ];

    fn try_from(file_name: &str) -> Option<Self> {
        Self::FILE_NAMES
            .iter()
            .find(|(name, _)| file_name == *name)
            .map(|(_, format)| *format)
    }
}

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum ConfigError {
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    UnknownFile(#[from] UnknownFile),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    ConflictingOptions(#[from] ConflictingOptions),
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

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
#[error("Cannot specify both 'dependency' and 'regex' for the same file: {path}")]
#[cfg_attr(
    feature = "miette",
    diagnostic(
        code(knope_versioning::versioned_file::conflicting_options),
        help(
            "Use 'dependency' to update a dependency version in a known file format, or 'regex' to match version strings in arbitrary text files, but not both."
        ),
        url("https://knope.tech/reference/config-file/packages#versioned_files")
    )
)]
pub struct ConflictingOptions {
    pub path: RelativePathBuf,
}

/// The configuration of a versioned file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    /// The location of the file
    pub(crate) path: RelativePathBuf,
    /// The type of file
    pub(crate) format: Format,
    /// If, within the file, we're versioning a dependency (not the entire package)
    pub dependency: Option<String>,
    /// If set, use regex pattern matching to find and replace the versions.
    pub regex: Option<Vec<String>>,
}

impl Config {
    /// Create a verified `Config` from a `RelativePathBuf`.
    ///
    /// # Errors
    ///
    /// If the file name does not match a supported format and no regex is provided
    /// If both dependency and regex are provided
    pub fn new(
        path: RelativePathBuf,
        dependency: Option<String>,
        regex: Option<Vec<String>>,
    ) -> Result<Self, ConfigError> {
        if dependency.is_some() && regex.is_some() {
            return Err(ConflictingOptions { path }.into());
        }

        if regex.is_some() {
            return Ok(Config {
                path,
                format: Format::RegexFile,
                dependency,
                regex,
            });
        }

        let Some(file_name) = path.file_name() else {
            return Err(UnknownFile { path }.into());
        };
        let Some(format) = Format::try_from(file_name) else {
            return Err(UnknownFile { path }.into());
        };
        Ok(Config {
            path,
            format,
            dependency,
            regex: None,
        })
    }

    #[must_use]
    pub fn as_path(&self) -> RelativePathBuf {
        self.path.clone()
    }

    #[must_use]
    pub fn to_pathbuf(&self) -> PathBuf {
        self.as_path().to_path("")
    }

    pub fn defaults() -> impl Iterator<Item = Self> {
        Format::FILE_NAMES
            .iter()
            .copied()
            .map(|(name, format)| Self {
                format,
                path: RelativePathBuf::from(name),
                dependency: None,
                regex: None,
            })
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
        self.path == *other
    }
}

impl PartialEq<Config> for RelativePathBuf {
    fn eq(&self, other: &Config) -> bool {
        other == self
    }
}
