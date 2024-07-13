use std::{
    borrow::{Borrow, Cow},
    fmt,
    fmt::Display,
    io::Write,
    mem::swap,
    ops::Deref,
    path::PathBuf,
};

use itertools::Itertools;
use knope_config::changelog_section::convert_to_versioning;
use knope_versioning::{
    changes::{Change, ChangeSource, CHANGESET_DIR, DEFAULT_PACKAGE_NAME},
    GoVersioning, Label, PackageNewError, Version, VersionedFile, VersionedFileError,
};
use miette::Diagnostic;
use serde::{Deserialize, Serialize};

use super::{
    changelog,
    changelog::Changelog,
    semver,
    semver::{bump, ConventionalRule},
    Release, Rule,
};
use crate::{
    config,
    dry_run::DryRun,
    fs,
    fs::read_to_string,
    integrations::git::{self, add_files},
    step::releases::{
        changelog::HeaderLevel,
        semver::UpdatePackageVersionError,
        versioned_file::{VersionFromSource, VersionSource},
    },
    workflow::Verbose,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Package {
    pub(crate) versioning: knope_versioning::Package,
    pub(crate) changelog: Option<Changelog>,
    pub(crate) name: Option<PackageName>,
    pub(crate) pending_changes: Vec<Change>,
    pub(crate) pending_tags: Vec<String>,
    pub(crate) prepared_release: Option<Release>,
    /// Version manually set by the caller to use instead of the one determined by semantic rule
    pub(crate) override_version: Option<Version>,
    pub(crate) assets: Option<Vec<Asset>>,
    pub(crate) go_versioning: GoVersioning,
}

impl Package {
    pub(crate) fn load(
        packages: Vec<config::Package>,
        git_tags: &[String],
        verbose: Verbose,
    ) -> Result<Vec<Self>, Error> {
        let packages = packages
            .into_iter()
            .map(|package| Package::validate(package, git_tags, verbose))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(packages)
    }

    fn validate(
        package: config::Package,
        git_tags: &[String],
        verbose: Verbose,
    ) -> Result<Self, Error> {
        if verbose == Verbose::Yes {
            if let Some(package_name) = &package.name {
                println!("Loading package {package_name}");
            } else {
                println!("Loading package");
            }
        }
        let versioned_files: Vec<VersionedFile> = package
            .versioned_files
            .iter()
            .map(|path| {
                let content = read_to_string(path.to_pathbuf())?;
                VersionedFile::new(path, content, git_tags).map_err(Error::VersionedFile)
            })
            .try_collect()?;
        if verbose == Verbose::Yes {
            for versioned_file in &versioned_files {
                println!(
                    "{} has version {}",
                    versioned_file.path(),
                    versioned_file.version(),
                );
            }
        }
        let versioning = knope_versioning::Package::new(
            versioned_files,
            convert_to_versioning(package.extra_changelog_sections),
            package.scopes,
        )?;
        Ok(Self {
            versioning,
            changelog: package
                .changelog
                .map(|path| path.to_path("").try_into())
                .transpose()?,
            name: package.name,
            assets: package.assets,
            go_versioning: if package.ignore_go_major_versioning {
                GoVersioning::IgnoreMajorRules
            } else {
                GoVersioning::default()
            },
            pending_changes: Vec::new(),
            pending_tags: Vec::new(),
            prepared_release: None,
            override_version: None,
        })
    }

    fn bump_rule(&self, verbose: Verbose) -> ConventionalRule {
        self.pending_changes
            .iter()
            .map(|change| {
                let rule = ConventionalRule::from(&change.change_type);
                if let Verbose::Yes = verbose {
                    println!(
                        "{change_source}\n\timplies rule {rule}",
                        change_source = change.original_source
                    );
                }
                rule
            })
            .max()
            .unwrap_or_default()
    }

    pub(crate) fn write_release(
        mut self,
        prerelease_label: &Option<Label>,
        git_tags: &[String],
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
            let versions = self.get_version(verbose, git_tags);
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
        let prepared_release = self.write_changelog(new_version.version, dry_run)?;
        let is_prerelease = prepared_release.version.is_prerelease();
        self.prepared_release = Some(prepared_release);
        self.stage_changes_to_git(is_prerelease, dry_run)?;

        Ok(self)
    }
    // TODO: Use actions for this?
    fn stage_changes_to_git(&self, is_prerelease: bool, dry_run: DryRun) -> Result<(), Error> {
        let changeset_path = PathBuf::from(CHANGESET_DIR);
        let paths = self
            .versioning
            .versioned_files()
            .iter()
            .map(|versioned_file| versioned_file.path().to_path(""))
            .chain(
                self.changelog
                    .as_ref()
                    .map(|changelog| changelog.path.clone()),
            )
            .chain(self.pending_changes.iter().filter_map(|change| {
                if is_prerelease {
                    None
                } else if let ChangeSource::ChangeFile(unique_id) = &change.original_source {
                    Some(changeset_path.join(unique_id.to_file_name()))
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

    /// Adds content from `release` to `Self::changelog` if it exists.
    pub fn write_changelog(
        &mut self,
        version: Version,
        dry_run: DryRun,
    ) -> Result<Release, crate::step::releases::changelog::Error> {
        let mut additional_tags = Vec::new();
        swap(&mut self.pending_tags, &mut additional_tags);
        let release = Release::new(
            version,
            &self.pending_changes,
            &self.versioning.changelog_sections,
            self.changelog
                .as_ref()
                .map_or(HeaderLevel::H2, |it| it.section_header_level),
            additional_tags,
        );

        if let Some(changelog) = self.changelog.as_mut() {
            changelog.add_release(&release, dry_run)?;
        }

        Ok(release)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
impl Package {
    pub(crate) fn default() -> Self {
        Self {
            versioning: knope_versioning::Package::new(
                vec![VersionedFile::new(
                    &knope_versioning::VersionedFilePath::new("Cargo.toml".into()).unwrap(),
                    r#"
                [package]
                name = "knope"
                version = "0.1.0""#
                        .to_string(),
                    &[""],
                )
                .unwrap()],
                knope_versioning::changelog::Sections::default(),
                None,
            )
            .unwrap(),
            changelog: None,
            name: None,
            pending_changes: vec![],
            pending_tags: vec![],
            prepared_release: None,
            override_version: None,
            assets: None,
            go_versioning: GoVersioning::default(),
        }
    }
}

impl Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.name.as_deref().unwrap_or(DEFAULT_PACKAGE_NAME)
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct PackageName(String);

impl Default for PackageName {
    fn default() -> Self {
        Self(DEFAULT_PACKAGE_NAME.to_string())
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

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
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
    VersionedFile(#[from] VersionedFileError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    UpdatePackageVersion(#[from] UpdatePackageVersionError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    New(#[from] PackageNewError),
}
