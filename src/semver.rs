use std::fmt::Display;

use color_eyre::eyre::WrapErr;
use color_eyre::eyre::{eyre, Result};
use semver::Identifier;
use serde::Deserialize;

use crate::{package_json, pyproject};

/// The various rules that can be used when bumping the current version of a project via
/// [`crate::step::Step::BumpVersion`].
#[derive(Debug, Deserialize)]
#[serde(tag = "rule", content = "value")]
pub(crate) enum Rule {
    Major,
    Minor,
    Pre(String),
    Release,
}

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
                version.pre = Vec::new();
                version
            }),
            Version::PyProject(mut version) => Version::PyProject({
                version.pre = Vec::new();
                version
            }),
            Version::Package(mut version) => Version::Package({
                version.pre = Vec::new();
                version
            }),
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Version::Cargo(v) | Version::PyProject(v) | Version::Package(v) => {
                write!(f, "{}", v.to_string())
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

fn bump(version: Version, rule: &Rule) -> Result<Version> {
    match rule {
        Rule::Major => version.run_on_inner(|mut v| {
            v.increment_major();
            Ok(v)
        }),
        Rule::Minor => version.run_on_inner(|mut v| {
            v.increment_minor();
            Ok(v)
        }),
        Rule::Patch => version.run_on_inner(|mut v| {
            v.increment_patch();
            Ok(v)
        }),
        Rule::Release => Ok(version.reset_pre()),
        Rule::Pre(prefix) => version.run_on_inner(|v| bump_pre(v, prefix)),
    }
}

fn bump_pre(mut version: semver::Version, prefix: &str) -> Result<semver::Version> {
    if version.pre.is_empty() {
        version.pre = vec![
            Identifier::AlphaNumeric(prefix.to_owned()),
            Identifier::Numeric(0),
        ];
        return Ok(version);
    } else if version.pre.len() != 2 {
        return Err(eyre!(
            "A prerelease version already exists but could not be incremented"
        ));
    }
    if let Some(Identifier::AlphaNumeric(existing_prefix)) = version.pre.get(0) {
        if existing_prefix != prefix {
            return Err(eyre!(
                "Found prefix {} which does not match provided prefix {}",
                existing_prefix,
                prefix
            ));
        }
    } else {
        return Err(eyre!(
            "A prerelease version already exists but could not be incremented"
        ));
    }
    if let Identifier::Numeric(pre_version) = version.pre.remove(1) {
        version.pre.insert(1, Identifier::Numeric(pre_version + 1));
        Ok(version)
    } else {
        Err(eyre!("No numeric pre component to bump"))
    }
}
