use std::{fmt, fmt::Display, io::Write, path::PathBuf};

use itertools::Itertools;
use knope_config::changelog_section::convert_to_versioning;
use knope_versioning::{
    changes::Change, package::Name, Action, CreateRelease, GoVersioning, Label, PackageNewError,
    Version, VersionedFile, VersionedFileError,
};
use miette::Diagnostic;
use serde::{Deserialize, Serialize};

use super::{
    changelog,
    changelog::Changelog,
    conventional_commits, semver,
    semver::{bump, ConventionalRule},
    CurrentVersions, Release, Rule,
};
use crate::{
    config,
    dry_run::DryRun,
    fs,
    fs::read_to_string,
    integrations::git::{self, add_files, get_current_versions_from_tags},
    step::{
        releases::{
            changelog::HeaderLevel,
            semver::UpdatePackageVersionError,
            versioned_file::{VersionFromSource, VersionSource},
        },
        PrepareRelease,
    },
    workflow::Verbose,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Package {
    pub(crate) version: Option<CurrentVersions>,
    pub(crate) versioning: knope_versioning::Package,
    pub(crate) changelog: Option<Changelog>,
    pub(crate) pending_changes: Vec<Change>,
    pub(crate) pending_actions: Vec<Action>,
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

    pub(crate) fn name(&self) -> &Name {
        &self.versioning.name
    }

    /// Get the current version of a package determined by the last tag for the package _and_ the
    /// version in versioned files. The version from files takes precedent over version from tag.
    ///
    /// This is cached, so anything that mutates version is expected to update it here as well!
    pub(crate) fn get_version(
        &mut self,
        verbose: Verbose,
        all_tags: &[String],
    ) -> &CurrentVersions {
        if self.version.is_none() {
            if let Verbose::Yes = verbose {
                println!("Looking for Git tags matching package name.");
            }
            let mut current_versions =
                get_current_versions_from_tags(self.name().as_custom(), verbose, all_tags);

            if let Some(version_from_files) = self.versioning.get_version() {
                current_versions.update_version(version_from_files.clone());
            }

            self.version = Some(current_versions);
        }
        #[allow(clippy::unwrap_used)] // This was just inserted up above!
        self.version.as_ref().unwrap()
    }

    fn validate(
        package: config::Package,
        git_tags: &[String],
        verbose: Verbose,
    ) -> Result<Self, Error> {
        if verbose == Verbose::Yes {
            if let Name::Custom(package_name) = &package.name {
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
            package.name,
            versioned_files,
            convert_to_versioning(package.extra_changelog_sections),
            package.scopes,
        )?;
        Ok(Self {
            versioning,
            changelog: package.changelog.map(TryInto::try_into).transpose()?,
            assets: package.assets,
            go_versioning: if package.ignore_go_major_versioning {
                GoVersioning::IgnoreMajorRules
            } else {
                GoVersioning::default()
            },
            pending_changes: Vec::new(),
            pending_actions: Vec::new(),
            override_version: None,
            version: None,
        })
    }

    fn bump_rule(changes: &[Change], verbose: Verbose) -> ConventionalRule {
        changes
            .iter()
            .map(|change| {
                let rule = ConventionalRule::from(&change.change_type);
                if let Verbose::Yes = verbose {
                    println!(
                        // TODO: Change this to use `log` or `tracing`, then move this to knope-versioning
                        "{change_source}\n\timplies rule {rule}",
                        change_source = change.original_source
                    );
                }
                rule
            })
            .max()
            .unwrap_or_default()
    }

    pub(crate) fn prepare_release(
        mut self,
        prepare_release: &PrepareRelease,
        all_tags: &[String],
        changeset: &[changesets::Release],
        verbose: Verbose,
        dry_run: DryRun,
    ) -> Result<Package, Error> {
        let PrepareRelease {
            prerelease_label,
            ignore_conventional_commits,
            ..
        } = prepare_release;

        let commit_messages = if *ignore_conventional_commits {
            Vec::new()
        } else {
            conventional_commits::get_conventional_commits_after_last_stable_version(
                &self.versioning.name,
                self.versioning.scopes.as_ref(),
                verbose,
                all_tags,
            )?
        };
        let changes = self.versioning.get_changes(changeset, &commit_messages);

        if changes.is_empty() {
            return Ok(self);
        }

        if let Verbose::Yes = verbose {
            if let Name::Custom(package_name) = &self.versioning.name {
                println!("Determining new version for {package_name}");
            }
        }

        self.pending_actions = self.versioning.apply_changes(&changes);
        // TODO: .filter_map(// apply non-release ones).collect();

        self.write_release(&changes, prerelease_label, all_tags, dry_run, verbose)
            .map_err(Error::from)
    }

    fn write_release(
        mut self,
        changes: &[Change],
        prerelease_label: &Option<Label>,
        git_tags: &[String],
        dry_run: DryRun,
        verbose: Verbose,
    ) -> Result<Self, Error> {
        let versions = self.get_version(verbose, git_tags).clone();
        let new_version = if let Some(version) = self.override_version.take() {
            if let Verbose::Yes = verbose {
                println!("Using overridden version {version}");
            }
            VersionFromSource {
                version,
                source: VersionSource::OverrideVersion,
            }
        } else {
            let bump_rule = Self::bump_rule(changes, verbose);
            let rule = if let Some(pre_label) = prerelease_label {
                Rule::Pre {
                    label: pre_label.clone(),
                    stable_rule: bump_rule,
                }
            } else {
                bump_rule.into()
            };
            let version = bump(versions.clone(), &rule, verbose)?;
            VersionFromSource {
                version,
                source: VersionSource::Calculated,
            }
        };

        let is_prerelease = new_version.version.is_prerelease();
        self = self
            .write_version(versions, new_version.clone(), dry_run)?
            .write_changelog(changes, new_version.version, dry_run)?;
        self.stage_changes_to_git(is_prerelease, dry_run)?;

        Ok(self)
    }

    /// Consumes a [`Package`], writing it back to the file it came from. Returns the new version
    /// that was written. Adds all modified package files to Git.
    ///
    /// If `dry_run` is `true`, the version won't be written to any files.
    pub(crate) fn write_version(
        mut self,
        mut current_versions: CurrentVersions,
        new_version: VersionFromSource,
        dry_run: DryRun,
    ) -> Result<Self, UpdatePackageVersionError> {
        let version_str = new_version.version.to_string();
        let go_versioning = match &new_version {
            VersionFromSource {
                source: VersionSource::OverrideVersion,
                ..
            } => GoVersioning::BumpMajor,
            _ => self.go_versioning,
        };
        let actions = self
            .versioning
            .clone()
            .set_version(&new_version.version, go_versioning)?;
        current_versions.update_version(new_version.version);
        self.version = Some(current_versions);
        for action in actions {
            match action {
                Action::WriteToFile { path, content } => {
                    fs::write(dry_run, &version_str, &path.to_path(""), content)?;
                }
                _ => self.pending_actions.push(action),
            }
        }
        Ok(self)
    }

    // TODO: Use actions for files being written to instead of versioned_file + changelog
    fn stage_changes_to_git(&self, is_prerelease: bool, dry_run: DryRun) -> Result<(), Error> {
        let paths = self
            .versioning
            .versioned_files()
            .iter()
            .map(VersionedFile::path)
            .chain(self.changelog.as_ref().map(|changelog| &changelog.path))
            .chain(self.pending_actions.iter().filter_map(|action| {
                if is_prerelease {
                    None
                } else if let Action::RemoveFile { path } = &action {
                    fs::remove_file(dry_run, &path.to_path(""))
                        .ok()
                        .map(|()| path)
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
                writeln!(stdio, "  {path}").map_err(fs::Error::Stdout)?;
            }
            Ok(())
        } else {
            add_files(&paths).map_err(Error::from)
        }
    }

    /// Adds content from `release` to `Self::changelog` if it exists.
    /// TODO: Use actions instead of this function
    fn write_changelog(
        mut self,
        changes: &[Change],
        version: Version,
        dry_run: DryRun,
    ) -> Result<Self, crate::step::releases::changelog::Error> {
        let release_header_level = self
            .changelog
            .as_ref()
            .map_or(HeaderLevel::H1, |changelog| changelog.release_header_level);
        let release = Release::new(
            &version,
            changes,
            &self.versioning.changelog_sections,
            release_header_level,
        )?;

        self.changelog = if let Some(changelog) = self.changelog {
            let (changelog, new_changes) = changelog.with_release(&release);

            fs::write(
                dry_run,
                &format!("\n{new_changes}\n"),
                &changelog.path.to_path(""),
                &changelog.content,
            )?;

            Some(changelog)
        } else {
            None
        };
        self.pending_actions.insert(
            0,
            Action::CreateRelease(CreateRelease {
                version,
                notes: release.body_at_h1(),
            }),
        );
        Ok(self)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
impl Package {
    pub(crate) fn default() -> Self {
        Self {
            versioning: knope_versioning::Package::new(
                Name::Default,
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
            pending_changes: vec![],
            pending_actions: vec![],
            override_version: None,
            assets: None,
            go_versioning: GoVersioning::default(),
            version: None,
        }
    }
}

/// So we can use it in an interactive select
impl Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
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
