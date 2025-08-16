use std::{
    borrow::{Borrow, Cow},
    fmt,
    fmt::{Debug, Display},
    ops::Deref,
};

use changesets::PackageChange;
use itertools::Itertools;
#[cfg(feature = "miette")]
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::debug;

use crate::{
    PackageNewError::CargoLockNoDependency,
    action::Action,
    changes::{
        CHANGESET_DIR, Change, ChangeSource, GitInfo,
        conventional_commit::{Commit, changes_from_commit_messages},
    },
    release_notes::{ReleaseNotes, TimeError},
    semver::{Label, PackageVersions, PreReleaseNotFound, Rule, StableRule, Version},
    versioned_file,
    versioned_file::{Config, Format, GoVersioning, SetError, VersionedFile, cargo},
};

#[derive(Clone, Debug)]
pub struct Package {
    pub name: Name,
    pub versions: PackageVersions,
    versioned_files: Vec<Config>,
    pub release_notes: ReleaseNotes,
    scopes: Option<Vec<String>>,
}

impl Package {
    /// Try and combine a bunch of versioned files into one logical package.
    ///
    /// # Errors
    ///
    /// There must be at least one versioned file, and all files must have the same version.
    pub fn new<S: AsRef<str> + Debug>(
        name: Name,
        git_tags: &[S],
        versioned_files_tracked: Vec<Config>,
        all_versioned_files: &[VersionedFile],
        release_notes: ReleaseNotes,
        scopes: Option<Vec<String>>,
    ) -> Result<Self, Box<NewError>> {
        let (versioned_files, version_from_files) =
            validate_versioned_files(versioned_files_tracked, all_versioned_files)?;

        debug!("Looking for Git tags matching package name.");
        let mut versions = PackageVersions::from_tags(name.as_custom(), git_tags);
        if let Some(version_from_files) = version_from_files {
            versions.update_version(version_from_files);
        }

        Ok(Self {
            name,
            versions,
            versioned_files,
            release_notes,
            scopes,
        })
    }

    /// Returns the actions that must be taken to set this package to the new version, along
    /// with the version it was set to.
    ///
    /// The version can either be calculated from a semver rule or specified manually.
    ///
    /// # Errors
    ///
    /// If the file is a `go.mod`, there are rules about what versions are allowed.
    ///
    /// If serialization of some sort fails, which is a bug, then this will return an error.
    ///
    /// If the [`Rule::Release`] is specified, but there is no current prerelease, that's an
    /// error too.
    pub fn bump_version(
        &mut self,
        version: &Version,
        go_versioning: GoVersioning,
        versioned_files: Vec<VersionedFile>,
    ) -> Result<Vec<VersionedFile>, BumpError> {
        versioned_files
            .into_iter()
            .map(|mut file| {
                let configs = self
                    .versioned_files
                    .iter()
                    .filter(|config| *config == file.path())
                    .collect_vec();
                for config in configs {
                    file = file
                        .set_version(version, config.dependency.as_deref(), go_versioning)
                        .map_err(BumpError::SetError)?;
                }
                Ok(file)
            })
            .collect()
    }

    #[must_use]
    pub fn get_changes<'a>(
        &self,
        changeset: impl IntoIterator<Item = (&'a PackageChange, Option<GitInfo>)>,
        commit_messages: &[Commit],
    ) -> Vec<Change> {
        changes_from_commit_messages(
            commit_messages,
            self.scopes.as_ref(),
            &self.release_notes.sections,
        )
        .chain(Change::from_changeset(changeset))
        .collect()
    }

    /// Apply changes to the package, updating the internal version and returning the list of
    /// actions to take to complete the changes.
    ///
    /// # Errors
    ///
    /// If the file is a `go.mod`, there are rules about what versions are allowed.
    ///
    /// If serialization of some sort fails, which is a bug, then this will return an error.
    pub fn apply_changes(
        &mut self,
        changes: &[Change],
        versioned_files: Vec<VersionedFile>,
        config: ChangeConfig,
    ) -> Result<(Vec<VersionedFile>, Vec<Action>), BumpError> {
        if let Name::Custom(package_name) = &self.name {
            debug!("Determining new version for {package_name}");
        }

        let (version, go_versioning) = match config {
            ChangeConfig::Force(version) => {
                debug!("Using overridden version {version}");
                (version, GoVersioning::BumpMajor)
            }
            ChangeConfig::Calculate {
                prerelease_label,
                go_versioning,
            } => {
                let stable_rule = StableRule::from(changes);
                let rule = if let Some(pre_label) = prerelease_label {
                    Rule::Pre {
                        label: pre_label.clone(),
                        stable_rule,
                    }
                } else {
                    stable_rule.into()
                };
                (self.versions.bump(rule)?, go_versioning)
            }
        };

        let updated = self.bump_version(&version, go_versioning, versioned_files)?;
        let mut actions: Vec<Action> = changes
            .iter()
            .filter_map(|change| {
                if let ChangeSource::ChangeFile { id } = &change.original_source {
                    if version.is_prerelease() {
                        None
                    } else {
                        Some(Action::RemoveFile {
                            path: RelativePathBuf::from(CHANGESET_DIR).join(id.to_file_name()),
                        })
                    }
                } else {
                    None
                }
            })
            .collect();

        actions.extend(
            self.release_notes
                .create_release(version, changes, &self.name)?,
        );

        Ok((updated, actions))
    }
}

/// Run through the provided versioned files and make sure they meet all requirements in context.
///
/// Returns the potentially modified versioned files (e.g., setting defaults for lockfiles) and
/// the package version according to those files (if any).
fn validate_versioned_files(
    versioned_files_tracked: Vec<Config>,
    all_versioned_files: &[VersionedFile],
) -> Result<(Vec<Config>, Option<Version>), Box<NewError>> {
    let relevant_files: Vec<(Config, &VersionedFile)> = versioned_files_tracked
        .into_iter()
        .map(|path| {
            all_versioned_files
                .iter()
                .find(|f| f.path() == &path)
                .ok_or_else(|| NewError::NotFound(path.as_path()))
                .map(|f| (path, f))
        })
        .collect::<Result<_, _>>()?;

    let mut first_with_version: Option<(&VersionedFile, Version)> = None;
    let mut validated_files = Vec::with_capacity(relevant_files.len());

    for (config, versioned_file) in relevant_files.clone() {
        let config = validate_dependency(config, &relevant_files)?;
        let is_dep = config.dependency.is_some();
        validated_files.push(config);
        if is_dep {
            // Dependencies don't have package versions
            continue;
        }
        let version = versioned_file.version().map_err(NewError::VersionedFile)?;
        debug!("{path} has version {version}", path = versioned_file.path());
        if let Some((first_versioned_file, first_version)) = first_with_version.as_ref() {
            if *first_version != version {
                return Err(NewError::InconsistentVersions {
                    first_path: first_versioned_file.path().clone(),
                    first_version: first_version.clone(),
                    second_path: versioned_file.path().clone(),
                    second_version: version,
                }
                .into());
            }
        } else {
            first_with_version = Some((versioned_file, version));
        }
    }

    Ok((
        validated_files,
        first_with_version.map(|(_, version)| version),
    ))
}

fn validate_dependency(
    mut config: Config,
    versioned_files: &[(Config, &VersionedFile)],
) -> Result<Config, Box<NewError>> {
    match (&config.format, config.dependency.is_some()) {
        (Format::Cargo | Format::PackageJson | Format::PackageLockJson, _)
        | (Format::CargoLock, true) => Ok(config),
        (Format::CargoLock, false) => {
            // `Cargo.lock` needs to target a dependency. If there is a `Cargo.toml` file which is
            // _not_ a dependency, we default to that one.
            let cargo_package_name = versioned_files
                .iter()
                .find_map(|(config, file)| match file {
                    VersionedFile::Cargo(file) if config.dependency.is_none() => {
                        cargo::name_from_document(&file.document)
                    }
                    _ => None,
                })
                .ok_or(CargoLockNoDependency)?;
            config.dependency = Some(cargo_package_name.to_string());
            Ok(config)
        }
        (_, true) => Err(NewError::UnsupportedDependency(
            config.path.file_name().unwrap_or_default().to_string(),
        )
        .into()),
        (_, false) => Ok(config),
    }
}

pub enum ChangeConfig {
    Force(Version),
    Calculate {
        prerelease_label: Option<Label>,
        go_versioning: GoVersioning,
    },
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum NewError {
    #[error(
        "Found inconsistent versions in package: {first_path} had {first_version} and {second_path} had {second_version}"
    )]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code = "knope_versioning::inconsistent_versions",
            url = "https://knope.tech/reference/concepts/package/#version",
            help = "All files in a package must have the same version"
        )
    )]
    InconsistentVersions {
        first_path: RelativePathBuf,
        first_version: Version,
        second_path: RelativePathBuf,
        second_version: Version,
    },
    #[error("Versioned file not found: {0}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code = "knope_versioning::package::versioned_file_not_found",
            help = "this is likely a bug, please report it",
            url = "https://github.com/knope-dev/knope/issues/new",
        )
    )]
    NotFound(RelativePathBuf),
    #[error("Dependencies are not supported in {0} files")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::package::unsupported_dependency),
            help("Dependencies aren't supported in every file type."),
            url("https://knope.tech/reference/config-file/packages#versioned_files")
        )
    )]
    UnsupportedDependency(String),
    #[error("Cargo.lock must specify a dependency")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code = "knope_versioning::package::cargo_lock_no_dependency",
            help = "To use `Cargo.lock` in `versioned_files`, you must either manually specify \
            `dependency` or define a `Cargo.toml` with a `package.name` in the same array.",
            url = "https://knope.tech/reference/config-file/packages/#cargolock"
        )
    )]
    CargoLockNoDependency,
    #[error("Packages must have at least one versioned file")]
    NoPackages,
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    VersionedFile(#[from] versioned_file::Error),
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Name {
    Custom(String),
    #[default]
    Default,
}

impl Name {
    const DEFAULT: &'static str = "default";

    #[must_use]
    pub fn as_custom(&self) -> Option<&str> {
        match self {
            Self::Custom(name) => Some(name),
            Self::Default => None,
        }
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Custom(name) => write!(f, "{name}"),
            Self::Default => write!(f, "{}", Self::DEFAULT),
        }
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        match self {
            Self::Custom(name) => name,
            Self::Default => Self::DEFAULT,
        }
    }
}

impl Deref for Name {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Custom(name) => name,
            Self::Default => Self::DEFAULT,
        }
    }
}

impl From<&str> for Name {
    fn from(name: &str) -> Self {
        Self::Custom(name.to_string())
    }
}

impl From<String> for Name {
    fn from(name: String) -> Self {
        Self::Custom(name)
    }
}

impl From<Cow<'_, str>> for Name {
    fn from(name: Cow<str>) -> Self {
        Self::Custom(name.into_owned())
    }
}

impl Borrow<str> for Name {
    fn borrow(&self) -> &str {
        match self {
            Self::Custom(name) => name,
            Self::Default => Self::DEFAULT,
        }
    }
}

impl PartialEq<String> for Name {
    fn eq(&self, str: &String) -> bool {
        str == self.as_ref()
    }
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum BumpError {
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    SetError(#[from] SetError),
    #[error(transparent)]
    PreReleaseNotFound(#[from] PreReleaseNotFound),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    Time(#[from] TimeError),
}
