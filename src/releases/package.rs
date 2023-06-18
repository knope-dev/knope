use std::{
    ffi::OsStr,
    fmt,
    fmt::Display,
    fs::{read_to_string, write},
    io::Write,
    path::{Path, PathBuf},
};

use indexmap::IndexMap;
use itertools::Itertools;
use log::trace;

use crate::{
    config::{ChangeLogSectionName, CommitFooter, CustomChangeType, Package as PackageConfig},
    git::add_files,
    releases::{
        cargo,
        changesets::DEFAULT_CHANGESET_PACKAGE_NAME,
        get_current_versions_from_tag, go, package_json, pyproject,
        semver::{bump, ConventionalRule, Label, Version},
        Change, Release, Rule,
    },
    step::{StepError, StepError::InvalidCargoToml},
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Package {
    pub(crate) versioned_files: Vec<VersionedFile>,
    pub(crate) changelog: Option<Changelog>,
    pub(crate) name: Option<String>,
    pub(crate) scopes: Option<Vec<String>>,
    pub(crate) extra_changelog_sections: IndexMap<ChangelogSectionSource, ChangeLogSectionName>,
    pub(crate) pending_changes: Vec<Change>,
    pub(crate) prepared_release: Option<Release>,
}

impl Package {
    pub(crate) fn new(config: PackageConfig, name: Option<String>) -> Result<Self, StepError> {
        let versioned_files = config
            .versioned_files
            .into_iter()
            .map(VersionedFile::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        let changelog = config.changelog.map(Changelog::try_from).transpose()?;
        let mut extra_changelog_sections = IndexMap::new();
        for section in config.extra_changelog_sections.unwrap_or_default() {
            for footer in section.footers {
                extra_changelog_sections
                    .insert(ChangelogSectionSource::from(footer), section.name.clone());
            }
            for change_type in section.types {
                extra_changelog_sections.insert(change_type.into(), section.name.clone());
            }
        }
        let default_extra_footer = CommitFooter::from("Changelog-Note");
        extra_changelog_sections
            .entry(default_extra_footer.into())
            .or_insert_with(|| ChangeLogSectionName::from("Notes"));
        Ok(Package {
            versioned_files,
            changelog,
            name,
            scopes: config.scopes,
            extra_changelog_sections,
            pending_changes: Vec::new(),
            prepared_release: None,
        })
    }

    fn bump_rule(&self) -> ConventionalRule {
        self.pending_changes
            .iter()
            .map(|change| change.change_type().into())
            .max()
            .unwrap_or_default()
    }

    pub(crate) fn write_release(
        mut self,
        prerelease_label: &Option<Label>,
        dry_run: &mut Option<Box<dyn Write>>,
    ) -> Result<Self, StepError> {
        if self.pending_changes.is_empty() {
            return Ok(self);
        }

        let bump_rule = self.bump_rule();
        let versions = self.get_version()?;
        let rule = if let Some(pre_label) = prerelease_label {
            Rule::Pre {
                label: pre_label.clone(),
                stable_rule: bump_rule,
            }
        } else {
            bump_rule.into()
        };
        let new_version = bump(versions, &rule)?;

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
            .and_then(|(pos, _)| ALL_PACKAGE_FORMATS.get(pos).copied())
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
                    .into_latest()
                    .unwrap_or_default()
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
        extra_changelog_sections: None,
    })
}

/// Includes some helper text for the user to understand how to use the config to define packages.
pub(crate) fn suggested_package_toml() -> Result<String, StepError> {
    let package = find_packages();
    if let Some(package) = package {
        Ok(format!(
            "Found the package metadata files {files} in the current directory. You may need to add this \
            to your knope.toml:\n\n```\n[package]\n{toml}```",
            files = package.versioned_files.iter().map(|path| path.to_string_lossy())
                .collect::<Vec<_>>()
                .join(", "),
            toml = toml::to_string(&package).or(Err(StepError::FailedTomlSerialization))?
        ))
    } else {
        Ok(format!(
            "No supported package managers found in current directory. \
                    The supported formats are {formats}. Here's how you might define a package for `Cargo.toml`:\
                    \n\n```\n[package]\nversioned_files = [\"Cargo.toml\"]\nchangelog = \"CHANGELOG.md\"\n```",
            formats = PACKAGE_FORMAT_FILE_NAMES.join(", ")
        ))
    }
}
