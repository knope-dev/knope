use std::{
    ffi::OsStr,
    fmt::Display,
    fs::read_to_string,
    path::{Path, PathBuf},
};

use enum_iterator::{all, Sequence};
use knope_versioning::Version;
use miette::Diagnostic;
use thiserror::Error;

use super::{cargo, git, go, package_json, pubspec_yaml, pyproject};
use crate::{dry_run::DryRun, step::releases::go::GoVersioning, workflow::Verbose};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VersionedFile {
    /// The type of file format that `content` is.
    pub(crate) format: PackageFormat,
    /// The path to the file that was parsed.
    pub(crate) path: PathBuf,
    /// The raw content of the package manager file so it doesn't have to be read again.
    pub(crate) content: String,
}

impl TryFrom<PathBuf> for VersionedFile {
    type Error = Error;

    fn try_from(path: PathBuf) -> Result<Self> {
        let format = PackageFormat::try_from(&path)?;
        let content = read_to_string(&path).map_err(|e| ErrorKind::Io(path.clone(), e))?;
        Ok(Self {
            format,
            path,
            content,
        })
    }
}

#[derive(Debug, Diagnostic, Error)]
#[error(transparent)]
#[diagnostic(transparent)]
pub(crate) struct Error(Box<ErrorKind>);

impl<T: Into<ErrorKind>> From<T> for Error {
    fn from(kind: T) -> Self {
        Self(Box::new(kind.into()))
    }
}

#[derive(Debug, Diagnostic, Error)]
enum ErrorKind {
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
        url("https://knope.tech/reference/config-file/packages/#versioned_files")
    )]
    VersionedFileFormat(PathBuf),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(#[from] git::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Go(#[from] go::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Cargo(#[from] cargo::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    PyProject(#[from] pyproject::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    PackageJson(#[from] package_json::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    PubSpecYaml(#[from] pubspec_yaml::Error),
}

type Result<T> = std::result::Result<T, Error>;

impl VersionedFile {
    pub(crate) fn get_version(&self, verbose: Verbose) -> Result<VersionFromSource> {
        self.format.get_version(&self.content, &self.path, verbose)
    }

    pub(crate) fn set_version(
        &mut self,
        dry_run: DryRun,
        version_str: &VersionFromSource,
        go_versioning: GoVersioning,
    ) -> Result<()> {
        self.content = self.format.set_version(
            dry_run,
            self.content.clone(),
            version_str,
            &self.path,
            go_versioning,
        )?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Sequence)]
pub(crate) enum PackageFormat {
    Cargo,
    Dart,
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
            .ok_or_else(|| ErrorKind::NotAFile(path.clone()))?;
        all::<PackageFormat>()
            .find(|&format| format.file_name() == file_name)
            .ok_or_else(|| Error::from(ErrorKind::VersionedFileFormat(path.clone())))
    }
}

impl PackageFormat {
    /// Get the version from `content` for package named `name` (if any name).
    /// `path` is used for error reporting.
    pub(crate) fn get_version(
        self,
        content: &str,
        path: &Path,
        verbose: Verbose,
    ) -> Result<VersionFromSource> {
        match self {
            PackageFormat::Cargo => cargo::get_version(content, path)
                .map_err(ErrorKind::Cargo)
                .map(|version| VersionFromSource {
                    version,
                    source: path.into(),
                }),
            PackageFormat::Poetry => pyproject::get_version(content, path)
                .map_err(ErrorKind::PyProject)
                .map(|version| VersionFromSource {
                    version,
                    source: path.into(),
                }),
            PackageFormat::JavaScript => package_json::get_version(content, path)
                .map_err(ErrorKind::PackageJson)
                .map(|version| VersionFromSource {
                    version,
                    source: path.into(),
                }),
            PackageFormat::Go { .. } => {
                go::get_version(content, path, verbose).map_err(ErrorKind::Go)
            }
            PackageFormat::Dart => pubspec_yaml::get_version(content, path)
                .map_err(ErrorKind::PubSpecYaml)
                .map(|version| VersionFromSource {
                    version,
                    source: path.into(),
                }),
        }
        .map_err(Error::from)
    }

    /// Consume the `content` and return a version of it which contains `new_version`.
    ///
    /// `path` is only used for error reporting.
    pub(crate) fn set_version(
        self,
        dry_run: DryRun,
        content: String,
        new_version: &VersionFromSource,
        path: &Path,
        go_versioning: GoVersioning,
    ) -> Result<String> {
        match self {
            PackageFormat::Cargo => {
                cargo::set_version(dry_run, content, &new_version.version, path)
                    .map_err(Error::from)
            }
            PackageFormat::Poetry => {
                pyproject::set_version(dry_run, content, &new_version.version, path)
                    .map_err(Error::from)
            }
            PackageFormat::JavaScript => {
                package_json::set_version(dry_run, &content, &new_version.version, path)
                    .map_err(Error::from)
            }
            PackageFormat::Go => {
                go::set_version_in_file(dry_run, &content, new_version, path, go_versioning)
                    .map_err(Error::from)
            }
            PackageFormat::Dart => {
                pubspec_yaml::set_version(dry_run, &content, &new_version.version, path)
                    .map_err(Error::from)
            }
        }
    }

    pub(crate) const fn file_name(self) -> &'static str {
        match self {
            PackageFormat::Cargo => "Cargo.toml",
            PackageFormat::Dart => "pubspec.yaml",
            PackageFormat::Go { .. } => "go.mod",
            PackageFormat::JavaScript => "package.json",
            PackageFormat::Poetry => "pyproject.toml",
        }
    }
}

/// A version and where it came from.
pub(crate) struct VersionFromSource {
    pub(crate) version: Version,
    pub(crate) source: VersionSource,
}

impl Display for VersionFromSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} from {}", self.version, self.source)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum VersionSource {
    OverrideVersion,
    GitTag(String),
    File(String),
    Default,
    Calculated,
}

impl Display for VersionSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionSource::OverrideVersion => write!(f, "--override-version option"),
            VersionSource::GitTag(tag) => write!(f, "git tag {tag}"),
            VersionSource::File(file) => write!(f, "file {file}"),
            VersionSource::Default => write!(f, "default—no matching tags detected"),
            VersionSource::Calculated => write!(f, "calculated by Knope"),
        }
    }
}

impl From<&Path> for VersionSource {
    fn from(path: &Path) -> Self {
        Self::File(path.display().to_string())
    }
}
