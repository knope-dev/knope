use std::fmt::Display;

use color_eyre::eyre::WrapErr;
use color_eyre::eyre::{eyre, Result};
use semver::Prerelease;
use serde::Deserialize;

use crate::{package_json, pyproject};

/// The various rules that can be used when bumping the current version of a project via
/// [`crate::step::Step::BumpVersion`].
#[derive(Debug, Deserialize)]
#[serde(tag = "rule", content = "value")]
pub(crate) enum Rule {
    Major,
    Minor,
    Patch,
    Pre(String),
    Release,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Version {
    Cargo(semver::Version),
    PyProject(semver::Version),
    Package(semver::Version),
}

impl Version {
    fn run_on_inner<F: FnOnce(semver::Version) -> Result<semver::Version>>(
        self,
        func: F,
    ) -> Result<Self> {
        Ok(match self {
            Version::Cargo(version) => Version::Cargo(func(version)?),
            Version::PyProject(version) => Version::PyProject(func(version)?),
            Version::Package(version) => Version::Package(func(version)?),
        })
    }

    fn reset_pre(self) -> Self {
        match self {
            Version::Cargo(mut version) => Version::Cargo({
                version.pre = Prerelease::EMPTY;
                version
            }),
            Version::PyProject(mut version) => Version::PyProject({
                version.pre = Prerelease::EMPTY;
                version
            }),
            Version::Package(mut version) => Version::Package({
                version.pre = Prerelease::EMPTY;
                version
            }),
        }
    }

    /// Is the current version's major component 0? Useful to apply special rules in the context of
    /// [Semantic Versioning](https://semver.org/#spec-item-4).
    fn is_0(&self) -> bool {
        match self {
            Version::Cargo(v) | Version::PyProject(v) | Version::Package(v) => v.major == 0,
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Version::Cargo(v) | Version::PyProject(v) | Version::Package(v) => {
                write!(f, "{}", v)
            }
        }
    }
}

pub(crate) fn bump_version(state: crate::State, rule: &Rule) -> Result<crate::State> {
    if let Ok(version) = get_version() {
        let version = bump(version, rule).wrap_err("While bumping version")?;
        set_version(version)?;
    }
    Ok(state)
}

pub(crate) fn get_version() -> Result<Version> {
    if let Some(cargo_version) = crate::cargo::get_version("Cargo.toml") {
        let version = semver::Version::parse(&cargo_version).wrap_err_with(|| {
            format!(
                "Found {} in Cargo.toml which is not a valid version",
                cargo_version
            )
        })?;
        Ok(Version::Cargo(version))
    } else if let Some(pyproject_version) = pyproject::get_version("pyproject.toml") {
        let version = semver::Version::parse(&pyproject_version).wrap_err_with(|| {
            format!(
                "Found {} in pyproject.toml which is not a valid version",
                pyproject_version
            )
        })?;
        Ok(Version::PyProject(version))
    } else if let Some(package_version) = package_json::get_version("package.json") {
        let version = semver::Version::parse(&package_version).wrap_err_with(|| {
            format!(
                "Found {} in package.json which is not a valid version",
                package_version
            )
        })?;
        Ok(Version::Package(version))
    } else {
        Err(eyre!("No supported metadata found to parse version from"))
    }
}

fn set_version(version: Version) -> Result<()> {
    match version {
        Version::Cargo(version) => crate::cargo::set_version("Cargo.toml", &version.to_string())
            .wrap_err("While bumping Cargo.toml"),
        Version::PyProject(version) => {
            pyproject::set_version("pyproject.toml", &version.to_string())
                .wrap_err("While bumping pyproject.toml")
        }
        Version::Package(version) => {
            package_json::set_version("package.json", &version.to_string())
                .wrap_err("While bumping package.json")
        }
    }
}

/// Apply a Rule to a Version, incrementing & resetting the correct components.
///
/// ### Versions 0.x
/// Versions with major component 0 have special meaning in Semantic Versioning and therefore have
/// different behavior:
/// 1. [`Rule::Major`] will bump the minor component.
/// 2. [`Rule::Minor`] will bump the patch component.
fn bump(version: Version, rule: &Rule) -> Result<Version> {
    let is_0 = version.is_0();
    match (rule, is_0) {
        (Rule::Major, false) => version.run_on_inner(|mut v| {
            v.major += 1;
            v.minor = 0;
            v.patch = 0;
            v.pre = Prerelease::EMPTY;
            Ok(v)
        }),
        (Rule::Minor, false) | (Rule::Major, true) => version.run_on_inner(|mut v| {
            v.minor += 1;
            v.patch = 0;
            v.pre = Prerelease::EMPTY;
            Ok(v)
        }),
        (Rule::Patch, _) | (Rule::Minor, true) => version.run_on_inner(|mut v| {
            v.patch += 1;
            v.pre = Prerelease::EMPTY;
            Ok(v)
        }),
        (Rule::Release, _) => Ok(version.reset_pre()),
        (Rule::Pre(prefix), _) => version.run_on_inner(|v| bump_pre(v, prefix)),
    }
}

#[cfg(test)]
mod test_bump {
    use super::*;

    #[test]
    fn major() {
        let version = Version::Cargo(semver::Version::new(1, 2, 3));
        let version = bump(version, &Rule::Major).unwrap();

        assert_eq!(version, Version::Cargo(semver::Version::new(2, 0, 0)));
    }

    #[test]
    fn major_0() {
        let version = Version::Cargo(semver::Version::new(0, 1, 2));
        let version = bump(version, &Rule::Major).unwrap();

        assert_eq!(version, Version::Cargo(semver::Version::new(0, 2, 0)));
    }

    #[test]
    fn minor() {
        let version = Version::Cargo(semver::Version::new(1, 2, 3));
        let version = bump(version, &Rule::Minor).unwrap();

        assert_eq!(version, Version::Cargo(semver::Version::new(1, 3, 0)));
    }

    #[test]
    fn minor_0() {
        let version = Version::Cargo(semver::Version::new(0, 1, 2));
        let version = bump(version, &Rule::Minor).unwrap();

        assert_eq!(version, Version::Cargo(semver::Version::new(0, 1, 3)));
    }

    #[test]
    fn patch() {
        let version = Version::Cargo(semver::Version::new(1, 2, 3));
        let version = bump(version, &Rule::Patch).unwrap();

        assert_eq!(version, Version::Cargo(semver::Version::new(1, 2, 4)));
    }

    #[test]
    fn patch_0() {
        let version = Version::Cargo(semver::Version::new(1, 2, 3));
        let version = bump(version, &Rule::Patch).unwrap();

        assert_eq!(version, Version::Cargo(semver::Version::new(1, 2, 4)));
    }

    #[test]
    fn pre() {
        let version = Version::Cargo(semver::Version::new(1, 2, 3));
        let version = bump(version, &Rule::Pre("rc".to_string())).unwrap();

        assert_eq!(
            version,
            Version::Cargo(semver::Version::parse("1.2.3-rc.0").unwrap())
        );
    }

    #[test]
    fn release() {
        let version = Version::Cargo(semver::Version::parse("1.2.3-rc.0").unwrap());
        let version = bump(version, &Rule::Release).unwrap();

        assert_eq!(version, Version::Cargo(semver::Version::new(1, 2, 3)));
    }
}

fn bump_pre(mut version: semver::Version, prefix: &str) -> Result<semver::Version> {
    if version.pre.is_empty() {
        version.pre = Prerelease::new(&format!("{}.0", prefix))?;
        return Ok(version);
    }

    let pre_string = version.pre.as_str();
    let parts = pre_string.split('.').collect::<Vec<_>>();

    if parts.len() != 2 {
        return Err(eyre!(
            "A prerelease version already exists but could not be incremented"
        ));
    }

    if parts[0] != prefix {
        return Err(eyre!(
            "Found prefix {} which does not match provided prefix {}",
            parts[0],
            prefix,
        ));
    }
    let pre_version = parts[1].parse::<u16>()?;
    version.pre = Prerelease::new(&format!("{}.{}", prefix, pre_version))?;
    Ok(version)
}
