use std::{
    borrow::{Borrow, Cow},
    fmt,
    fmt::Display,
    io::Write,
    ops::Deref,
    path::PathBuf,
};

use indexmap::IndexMap;
use itertools::Itertools;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};

use super::{
    changelog,
    changelog::Changelog,
    changesets::DEFAULT_CHANGESET_PACKAGE_NAME,
    semver,
    semver::{bump, ConventionalRule, Label, Version},
    versioned_file,
    versioned_file::{VersionedFile, PACKAGE_FORMAT_FILE_NAMES},
    Change, Release, Rule,
};
use crate::{
    config::{ChangeLogSectionName, CommitFooter, CustomChangeType},
    dry_run::DryRun,
    fs,
    integrations::git::{self, add_files},
    step::releases::versioned_file::{VersionFromSource, VersionSource},
    workflow::Verbose,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Package {
    pub(crate) versioned_files: Vec<VersionedFile>,
    pub(crate) changelog: Option<Changelog>,
    pub(crate) name: Option<PackageName>,
    pub(crate) scopes: Option<Vec<String>>,
    pub(crate) extra_changelog_sections: IndexMap<ChangelogSectionSource, ChangeLogSectionName>,
    pub(crate) pending_changes: Vec<Change>,
    pub(crate) prepared_release: Option<Release>,
    /// Version manually set by the caller to use instead of the one determined by semantic rule
    pub(crate) override_version: Option<Version>,
    pub(crate) assets: Option<Vec<Asset>>,
}

impl Package {
    fn bump_rule(&self, verbose: Verbose) -> ConventionalRule {
        self.pending_changes
            .iter()
            .map(|change| {
                let rule = change.change_type().into();
                let change_source = match change {
                    Change::ConventionalCommit(_) => "commit",
                    Change::ChangeSet(_) => "changeset",
                };
                if let Verbose::Yes = verbose {
                    println!("{change_source} {change}\n\timplies rule {rule}");
                }
                rule
            })
            .max()
            .unwrap_or_default()
    }

    pub(crate) fn write_release(
        mut self,
        prerelease_label: &Option<Label>,
        dry_run: DryRun,
        verbose: Verbose,
    ) -> Result<Self, Error> {
        if self.pending_changes.is_empty() {
            return Ok(self);
        }

        if let Verbose::Yes = verbose {
            if let Some(package_name) = &self.name {
                println!("Determining new version for {package_name}");
            }
        }

        let new_version = if let Some(version) = self.override_version.take() {
            if let Verbose::Yes = verbose {
                println!("Using overridden version {version}");
            }
            VersionFromSource {
                version,
                source: VersionSource::OverrideVersion,
            }
        } else {
            let versions = self.get_version(verbose)?;
            let bump_rule = self.bump_rule(verbose);
            let rule = if let Some(pre_label) = prerelease_label {
                Rule::Pre {
                    label: pre_label.clone(),
                    stable_rule: bump_rule,
                }
            } else {
                bump_rule.into()
            };
            let version = bump(versions, &rule, verbose)?;
            VersionFromSource {
                version,
                source: VersionSource::Calculated,
            }
        };

        self = self.write_version(&new_version, dry_run)?;
        self.prepared_release = Some(self.write_changelog(new_version.version, dry_run)?);
        self.stage_changes_to_git(dry_run)?;

        Ok(self)
    }
    fn stage_changes_to_git(&self, dry_run: DryRun) -> Result<(), Error> {
        let changeset_path = PathBuf::from(".changeset");
        let paths = self
            .versioned_files
            .iter()
            .map(|versioned_file| versioned_file.path.clone())
            .chain(
                self.changelog
                    .as_ref()
                    .map(|changelog| changelog.path.clone()),
            )
            .chain(self.pending_changes.iter().filter_map(|change| {
                if let Change::ChangeSet(change) = change {
                    Some(changeset_path.join(change.unique_id.to_file_name()))
                } else {
                    None
                }
            }))
            .collect_vec();
        if paths.is_empty() {
            Ok(())
        } else if let Some(stdio) = dry_run {
            writeln!(stdio, "Would add files to git:").map_err(fs::Error::Stdout)?;
            for path in &paths {
                writeln!(stdio, "  {}", path.display()).map_err(fs::Error::Stdout)?;
            }
            Ok(())
        } else {
            add_files(&paths).map_err(Error::from)
        }
    }
}

impl Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.name
                .as_deref()
                .unwrap_or(DEFAULT_CHANGESET_PACKAGE_NAME)
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct PackageName(String);

impl Display for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for PackageName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&str> for PackageName {
    fn from(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl From<String> for PackageName {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl From<Cow<'_, str>> for PackageName {
    fn from(name: Cow<str>) -> Self {
        Self(name.into_owned())
    }
}

impl Borrow<str> for PackageName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Asset {
    pub(crate) path: PathBuf,
    name: Option<String>,
}

impl Asset {
    pub(crate) fn name(&self) -> Result<String, AssetNameError> {
        if let Some(name) = &self.name {
            Ok(name.clone())
        } else {
            self.path
                .file_name()
                .ok_or(AssetNameError {
                    path: self.path.clone(),
                })
                .map(|name| name.to_string_lossy().into_owned())
        }
    }
}

#[derive(Debug, Diagnostic, thiserror::Error)]
#[error("No asset name set, and name could not be determined from path {path}")]
pub(crate) struct AssetNameError {
    path: PathBuf,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum ChangelogSectionSource {
    CommitFooter(CommitFooter),
    CustomChangeType(CustomChangeType),
}

impl From<CommitFooter> for ChangelogSectionSource {
    fn from(footer: CommitFooter) -> Self {
        Self::CommitFooter(footer)
    }
}

impl From<CustomChangeType> for ChangelogSectionSource {
    fn from(change_type: CustomChangeType) -> Self {
        Self::CustomChangeType(change_type)
    }
}

impl Display for ChangelogSectionSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommitFooter(footer) => footer.fmt(f),
            Self::CustomChangeType(change_type) => change_type.fmt(f),
        }
    }
}

/// Find all supported package formats in the current directory.
pub(crate) fn find_packages() -> Result<Package, Error> {
    let default = PathBuf::from("CHANGELOG.md");
    let changelog = default
        .exists()
        .then(|| Changelog::try_from(default))
        .transpose()?;

    let versioned_files = PACKAGE_FORMAT_FILE_NAMES
        .iter()
        .filter_map(|name| {
            let path = PathBuf::from(name);
            if path.exists() {
                Some(VersionedFile::try_from(path))
            } else {
                None
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Package {
        versioned_files,
        changelog,
        ..Package::default()
    })
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    VersionedFile(#[from] versioned_file::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Changelog(#[from] changelog::Error),
    #[error("Could not serialize generated TOML")]
    #[diagnostic(
        code(releases::package::could_not_serialize_toml),
        help("This is a bug, please report it to https://github.com/knope-dev/knope")
    )]
    GeneratedTOML(#[from] toml::ser::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Semver(#[from] semver::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    InvalidPreReleaseVersion(#[from] semver::InvalidPreReleaseVersion),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(#[from] git::Error),
    #[error("No packages are defined")]
    #[diagnostic(
        code(package::no_defined_packages),
        help("You must define at least one [package] in knope.toml. {package_suggestion}"),
        url("https://knope-dev.github.io/knope/config/packages.html")
    )]
    NoDefinedPackages { package_suggestion: String },
}

impl Error {
    pub fn no_defined_packages_with_help() -> Self {
        match suggested_package_toml() {
            Ok(help) => Self::NoDefinedPackages {
                package_suggestion: help,
            },
            Err(err) => err,
        }
    }
}

/// Includes some helper text for the user to understand how to use the config to define packages.
pub(crate) fn suggested_package_toml() -> Result<String, Error> {
    let package = find_packages()?;
    if package.versioned_files.is_empty() {
        Ok(format!(
            "No supported package managers found in current directory. \
                    The supported formats are {formats}. Here's how you might define a package for `Cargo.toml`:\
                    \n\n```\n[package]\nversioned_files = [\"Cargo.toml\"]\nchangelog = \"CHANGELOG.md\"\n```",
            formats = PACKAGE_FORMAT_FILE_NAMES.join(", ")
        ))
    } else {
        let toml = crate::config::toml::Package::from(package);
        let toml = toml::to_string_pretty(&toml)?;
        Ok(format!(
            "Found some package metadata files in the current directory. You may need to add this \
            to your knope.toml:\n\n```\n[package]\n{toml}```",
        ))
    }
}
