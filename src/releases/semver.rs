use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::{read_to_string, write};
use std::path::Path;

use semver::{Prerelease, Version};
use serde::{Deserialize, Serialize};

use crate::{RunType, state};
use crate::step::StepError;

use super::{cargo, Package, package_json, pyproject};

/// The various rules that can be used when bumping the current version of a project via
/// [`crate::step::Step::BumpVersion`].
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "rule")]
pub(crate) enum Rule {
    Major,
    Minor,
    Patch,
    Pre {
        label: String,
        #[serde(skip)]
        fallback_rule: ConventionalRule,
    },
    Release,
}

impl From<ConventionalRule> for Rule {
    fn from(fallback_rule: ConventionalRule) -> Self {
        match fallback_rule {
            ConventionalRule::Major => Rule::Major,
            ConventionalRule::Minor => Rule::Minor,
            ConventionalRule::Patch => Rule::Patch,
        }
    }
}

/// The rules that can be derived from Conventional Commits.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ConventionalRule {
    Major,
    Minor,
    Patch,
}

impl Default for ConventionalRule {
    fn default() -> Self {
        ConventionalRule::Patch
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct PackageVersion<'path> {
    /// The version that was parsed from the package manager file.
    pub(crate) version: Version,
    /// The type of file format that `content` is.
    pub(crate) format: PackageFormat,
    /// The path to the file that was parsed.
    path: &'path Path,
    /// The raw content of the package manager file so it doesn't have to be read again.
    content: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PackageFormat {
    Cargo,
    Poetry,
    JavaScript,
}

impl Display for PackageVersion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.version)
    }
}

pub(super) fn bump_version(
    rule: Rule,
    dry_run: bool,
    packages: &[Package],
) -> Result<Version, StepError> {
    let mut package_version = get_version(packages)?;
    package_version.version = bump(package_version.version, rule)?;
    if dry_run {
        Ok(package_version.version)
    } else {
        set_version(package_version)
    }
}

pub(crate) fn bump_version_and_update_state(
    run_type: RunType,
    rule: Rule,
) -> Result<RunType, StepError> {
    match run_type {
        RunType::DryRun {
            mut state,
            mut stdout,
        } => {
            let version = bump_version(rule, true, &state.packages)?;
            writeln!(stdout, "Would bump version to {}", version)?;
            state.release = state::Release::Bumped(version);
            Ok(RunType::DryRun { state, stdout })
        }
        RunType::Real(mut state) => {
            let version = bump_version(rule, false, &state.packages)?;
            state.release = state::Release::Bumped(version);
            Ok(RunType::Real(state))
        }
    }
}

pub(crate) fn get_version(packages: &[Package]) -> Result<PackageVersion, StepError> {
    if packages.is_empty() {
        return Err(StepError::no_defined_packages_with_help());
    }
    if packages.len() > 1 {
        return Err(StepError::TooManyPackages);
    }
    let package = &packages[0];
    if package.versioned_files.is_empty() {
        return Err(StepError::NoVersionedFiles);
    }
    if package.versioned_files.len() > 1 {
        return Err(StepError::TooManyVersionedFiles);
    }
    let versioned_file = &package.versioned_files[0];

    let content = read_to_string(versioned_file)
        .map_err(|_| StepError::FileNotFound(versioned_file.clone()))?;

    let file_name = versioned_file
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| StepError::FileNotFound(versioned_file.clone()))?;

    let (version_string, format) = match file_name {
        "Cargo.toml" => (cargo::get_version(&content)?, PackageFormat::Cargo),
        "pyproject.toml" => (pyproject::get_version(&content)?, PackageFormat::Poetry),
        "package.json" => (
            package_json::get_version(&content)?,
            PackageFormat::JavaScript,
        ),
        other => {
            return Err(StepError::VersionedFileFormat(String::from(other)));
        }
    };
    let version =
        Version::parse(&version_string).map_err(|_| StepError::InvalidSemanticVersion {
            version: version_string,
            file_name: versioned_file.clone(),
        })?;

    Ok(PackageVersion {
        version,
        path: versioned_file.as_path(),
        content,
        format,
    })
}

/// Consumes a [`PackageVersion`], writing it back to the file it came from. Returns the new version
/// that was written.
fn set_version(version: PackageVersion) -> Result<Version, StepError> {
    let PackageVersion {
        version,
        format,
        content,
        path,
    } = version;
    let new_content = match format {
        PackageFormat::Cargo => cargo::set_version(content, &version.to_string()),
        PackageFormat::Poetry => pyproject::set_version(content, &version.to_string()),
        PackageFormat::JavaScript => package_json::set_version(&content, &version.to_string()),
    }?;
    write(path, &new_content).map_err(|_| StepError::FileNotFound(path.to_path_buf()))?;
    Ok(version)
}

/// Apply a Rule to a [`PackageVersion`], incrementing & resetting the correct components.
///
/// ### Versions 0.x
///
/// Versions with major component 0 have special meaning in Semantic Versioning and therefore have
/// different behavior:
/// 1. [`Rule::Major`] will bump the minor component.
/// 2. [`Rule::Minor`] will bump the patch component.
fn bump(mut version: Version, rule: Rule) -> Result<Version, StepError> {
    let is_0 = version.major == 0;
    match (rule, is_0) {
        (Rule::Major, false) => {
            version.major += 1;
            version.minor = 0;
            version.patch = 0;
            version.pre = Prerelease::EMPTY;
            Ok(version)
        }
        (Rule::Minor, false) | (Rule::Major, true) => {
            version.minor += 1;
            version.patch = 0;
            version.pre = Prerelease::EMPTY;
            Ok(version)
        }
        (Rule::Patch, _) | (Rule::Minor, true) => {
            version.patch += 1;
            version.pre = Prerelease::EMPTY;
            Ok(version)
        }
        (Rule::Release, _) => {
            version.pre = Prerelease::EMPTY;
            Ok(version)
        }
        (
            Rule::Pre {
                label: prefix,
                fallback_rule,
            },
            _,
        ) => bump_pre(version, &prefix, fallback_rule),
    }
}

#[cfg(test)]
mod test_bump {
    use super::*;

    #[test]
    fn major() {
        let version = Version::new(1, 2, 3);
        let version = bump(version, Rule::Major).unwrap();

        assert_eq!(version, Version::new(2, 0, 0));
    }

    #[test]
    fn major_0() {
        let version = Version::new(0, 1, 2);
        let version = bump(version, Rule::Major).unwrap();

        assert_eq!(version, Version::new(0, 2, 0));
    }

    #[test]
    fn minor() {
        let version = Version::new(1, 2, 3);
        let version = bump(version, Rule::Minor).unwrap();

        assert_eq!(version, Version::new(1, 3, 0));
    }

    #[test]
    fn minor_0() {
        let version = Version::new(0, 1, 2);
        let version = bump(version, Rule::Minor).unwrap();

        assert_eq!(version, Version::new(0, 1, 3));
    }

    #[test]
    fn patch() {
        let version = Version::new(1, 2, 3);
        let version = bump(version, Rule::Patch).unwrap();

        assert_eq!(version, Version::new(1, 2, 4));
    }

    #[test]
    fn patch_0() {
        let version = Version::new(1, 2, 3);
        let version = bump(version, Rule::Patch).unwrap();

        assert_eq!(version, Version::new(1, 2, 4));
    }

    #[test]
    fn pre() {
        let version = Version::new(1, 2, 3);
        let version = bump(
            version,
            Rule::Pre {
                label: String::from("rc"),
                fallback_rule: ConventionalRule::Minor,
            },
        )
        .unwrap();

        assert_eq!(version, Version::parse("1.3.0-rc.0").unwrap());
    }

    #[test]
    fn release() {
        let version = Version::parse("1.2.3-rc.0").unwrap();
        let version = bump(version, Rule::Release).unwrap();

        assert_eq!(version, Version::new(1, 2, 3));
    }
}

/// Bumps the pre-release component of a [`Version`].
///
/// If the existing [`Version`] has no pre-release,
/// `semantic_rule` will be used to bump to primary components before the
/// pre-release component is added.
///
/// # Errors
///
/// Can fail if there is an existing pre-release component that can't be incremented.
fn bump_pre(
    mut version: Version,
    prefix: &str,
    fallback_rule: ConventionalRule,
) -> Result<Version, StepError> {
    if version.pre.is_empty() {
        let mut version = bump(version, fallback_rule.into())?;
        let pre_release_version = format!("{}.0", prefix);
        version.pre = Prerelease::new(&pre_release_version)
            .map_err(|_| StepError::InvalidPreReleaseVersion(pre_release_version))?;
        return Ok(version);
    }

    let pre_string = version.pre.as_str();
    let parts = pre_string.split('.').collect::<Vec<_>>();

    if parts.len() != 2 {
        return Err(StepError::InvalidPreReleaseVersion(String::from(
            pre_string,
        )));
    }

    if parts[0] != prefix {
        let pre_release_version = format!("{}.0", prefix);
        version.pre = Prerelease::new(&pre_release_version)
            .map_err(|_| StepError::InvalidPreReleaseVersion(pre_release_version))?;
        return Ok(version);
    }
    let pre_version = parts[1]
        .parse::<u16>()
        .map_err(|_| StepError::InvalidPreReleaseVersion(String::from(pre_string)))?;
    let pre_release_version = format!("{}.{}", prefix, pre_version + 1);
    version.pre = Prerelease::new(&pre_release_version)
        .map_err(|_| StepError::InvalidPreReleaseVersion(pre_release_version))?;
    Ok(version)
}
