use std::{
    borrow::{Borrow, Cow},
    collections::{HashMap, HashSet},
    fmt,
    fmt::Display,
    io::Write,
    ops::Deref,
    path::PathBuf,
};

use enum_iterator::all;
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
    versioned_file::VersionedFile,
    workspace, Change, Release, Rule,
};
use crate::{
    config::{
        toml::package::ChangelogSection, ChangeLogSectionName, CommitFooter, CustomChangeType,
    },
    dry_run::DryRun,
    fs,
    integrations::git::{self, add_files},
    step::releases::{
        go::GoVersioning,
        versioned_file::{PackageFormat, VersionFromSource, VersionSource},
        workspace::check_for_workspaces,
    },
    workflow::Verbose,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Package {
    pub(crate) versioned_files: Vec<VersionedFile>,
    pub(crate) changelog: Option<Changelog>,
    pub(crate) changelog_sections: ChangelogSections,
    pub(crate) name: Option<PackageName>,
    pub(crate) scopes: Option<Vec<String>>,
    pub(crate) pending_changes: Vec<Change>,
    pub(crate) prepared_release: Option<Release>,
    /// Version manually set by the caller to use instead of the one determined by semantic rule
    pub(crate) override_version: Option<Version>,
    pub(crate) assets: Option<Vec<Asset>>,
    pub(crate) go_versioning: GoVersioning,
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

    pub(crate) fn new(name: String, versioned_files: Vec<VersionedFile>) -> Self {
        Self {
            name: Some(PackageName(name.clone())),
            versioned_files,
            scopes: Some(vec![name]),
            ..Self::default()
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

impl Default for PackageName {
    fn default() -> Self {
        Self(DEFAULT_CHANGESET_PACKAGE_NAME.to_string())
    }
}

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ChangelogSections(IndexMap<ChangelogSectionSource, ChangeLogSectionName>);

impl Default for ChangelogSections {
    fn default() -> Self {
        let mut changelog_sections = IndexMap::new();
        changelog_sections.insert(
            ChangelogSectionSource::CustomChangeType(CustomChangeType::from("major")),
            ChangeLogSectionName::from("Breaking Changes"),
        );
        changelog_sections.insert(
            ChangelogSectionSource::CustomChangeType(CustomChangeType::from("minor")),
            ChangeLogSectionName::from("Features"),
        );
        changelog_sections.insert(
            ChangelogSectionSource::CustomChangeType(CustomChangeType::from("patch")),
            ChangeLogSectionName::from("Fixes"),
        );
        changelog_sections.insert(CommitFooter::from("Changelog-Note").into(), "Notes".into());
        Self(changelog_sections)
    }
}

impl From<Vec<ChangelogSection>> for ChangelogSections {
    fn from(sections_from_toml: Vec<ChangelogSection>) -> Self {
        let mut changelog_sections = Self::default();

        for section in sections_from_toml {
            for footer in section.footers {
                changelog_sections.remove(&ChangelogSectionSource::from(footer.clone()));
                changelog_sections.insert(
                    ChangelogSectionSource::CommitFooter(footer),
                    section.name.clone(),
                );
            }
            for change_type in section.types {
                changelog_sections.remove(&ChangelogSectionSource::from(change_type.clone()));
                changelog_sections.insert(change_type.into(), section.name.clone());
            }
        }
        changelog_sections
    }
}

impl From<ChangelogSections> for Vec<ChangelogSection> {
    fn from(mut sections: ChangelogSections) -> Self {
        let defaults = ChangelogSections::default();
        for (source, name) in defaults {
            if sections.get(&source).is_some_and(|it| *it == name) {
                sections.remove(&source);
            }
        }
        let mut footers = HashMap::new();
        let mut types = HashMap::new();
        let mut section_names = HashSet::new();
        for (source, name) in sections.0 {
            section_names.insert(name.clone());
            match source {
                ChangelogSectionSource::CommitFooter(footer) => {
                    footers.entry(name).or_insert_with(Vec::new).push(footer);
                }
                ChangelogSectionSource::CustomChangeType(change_type) => {
                    types.entry(name).or_insert_with(Vec::new).push(change_type);
                }
            }
        }
        section_names
            .into_iter()
            .map(|name| ChangelogSection {
                footers: footers.remove(&name).unwrap_or_default(),
                types: types.remove(&name).unwrap_or_default(),
                name,
            })
            .collect()
    }
}

impl IntoIterator for ChangelogSections {
    type Item = (ChangelogSectionSource, ChangeLogSectionName);
    type IntoIter = indexmap::map::IntoIter<ChangelogSectionSource, ChangeLogSectionName>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl ChangelogSections {
    fn remove(&mut self, source: &ChangelogSectionSource) {
        self.0.shift_remove(source);
    }

    fn insert(&mut self, source: ChangelogSectionSource, name: ChangeLogSectionName) {
        self.0.insert(source, name);
    }

    pub(crate) fn get(&self, source: &ChangelogSectionSource) -> Option<&ChangeLogSectionName> {
        self.0.get(source)
    }

    pub(crate) fn contains_key(&self, source: &ChangelogSectionSource) -> bool {
        self.0.contains_key(source)
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &ChangeLogSectionName> {
        self.0.values()
    }

    pub(crate) fn into_keys(
        self,
    ) -> indexmap::map::IntoKeys<ChangelogSectionSource, ChangeLogSectionName> {
        self.0.into_keys()
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
pub(crate) fn find_packages() -> Result<Vec<Package>, Error> {
    let packages = check_for_workspaces()?;

    if !packages.is_empty() {
        return Ok(packages);
    }

    let default_changelog_path = PathBuf::from("CHANGELOG.md");
    let changelog = default_changelog_path
        .exists()
        .then(|| Changelog::try_from(default_changelog_path))
        .transpose()?;

    let versioned_files = all::<PackageFormat>()
        .filter_map(|format| {
            let path = PathBuf::from(format.file_name());
            if path.exists() {
                Some(VersionedFile::try_from(path))
            } else {
                None
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    if versioned_files.is_empty() {
        Ok(vec![])
    } else {
        Ok(vec![Package {
            versioned_files,
            changelog,
            ..Package::default()
        }])
    }
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
    #[error("No packages to operate on")]
    #[diagnostic(
        code(package::no_defined_packages),
        help("There must be at least one package for Knope to work with, no supported package files were found in this directory."),
        url("https://knope.tech/reference/config-file/packages/")
    )]
    NoDefinedPackages,
    #[error(transparent)]
    #[diagnostic(transparent)]
    Workspace(#[from] workspace::Error),
}
