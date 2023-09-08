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

use crate::{
    config::{ChangeLogSectionName, CommitFooter, CustomChangeType},
    git::add_files,
    releases::{
        changelog::Changelog,
        changesets::DEFAULT_CHANGESET_PACKAGE_NAME,
        semver::{bump, ConventionalRule, Label, Version},
        versioned_file::{VersionedFile, PACKAGE_FORMAT_FILE_NAMES},
        Change, Release, Rule,
    },
    step::StepError,
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
        dry_run: &mut Option<Box<dyn Write>>,
        verbose: Verbose,
    ) -> Result<Self, StepError> {
        if self.pending_changes.is_empty() {
            return Ok(self);
        }

        if let Verbose::Yes = verbose {
            if let Some(package_name) = &self.name {
                println!("Determining new version for {package_name}");
            }
        }

        let new_version = if let Some(override_version) = self.override_version.take() {
            if let Verbose::Yes = verbose {
                println!("Using overridden version {override_version}");
            }
            override_version
        } else {
            let versions = self.get_version()?;
            let bump_rule = self.bump_rule(verbose);
            let rule = if let Some(pre_label) = prerelease_label {
                Rule::Pre {
                    label: pre_label.clone(),
                    stable_rule: bump_rule,
                }
            } else {
                bump_rule.into()
            };
            bump(versions, &rule, verbose)?
        };

        self = self.write_version(&new_version, dry_run)?;
        self.prepared_release = Some(self.write_changelog(new_version, dry_run)?);
        self.stage_changes_to_git(dry_run)?;

        Ok(self)
    }
    fn stage_changes_to_git(&self, dry_run: &mut Option<Box<dyn Write>>) -> Result<(), StepError> {
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
        } else if let Some(stdio) = dry_run.as_deref_mut() {
            writeln!(stdio, "Would add files to git:")?;
            for path in &paths {
                writeln!(stdio, "  {}", path.display())?;
            }
            Ok(())
        } else {
            add_files(&paths)
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
pub(crate) fn find_packages() -> Result<Package, StepError> {
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

/// Includes some helper text for the user to understand how to use the config to define packages.
pub(crate) fn suggested_package_toml() -> Result<String, StepError> {
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
