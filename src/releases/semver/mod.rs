use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub(crate) use version::{Label, Prerelease, Version};

use crate::git::add_files;
use crate::releases::git::get_current_versions_from_tag;
use crate::releases::package::Package;
use crate::releases::{CurrentVersions, Prereleases};
use crate::step::StepError;
use crate::{state, RunType};

mod version;

/// The various rules that can be used when bumping the current version of a project via
/// [`crate::step::Step::BumpVersion`].
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "rule")]
pub(crate) enum Rule {
    Major,
    Minor,
    Patch,
    Pre {
        label: Label,
        #[serde(skip)]
        stable_rule: ConventionalRule,
    },
    Release,
}

impl From<ConventionalRule> for Rule {
    fn from(conventional_rule: ConventionalRule) -> Self {
        match conventional_rule {
            ConventionalRule::Major => Rule::Major,
            ConventionalRule::Minor => Rule::Minor,
            ConventionalRule::Patch => Rule::Patch,
        }
    }
}

/// The rules that can be derived from Conventional Commits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
pub(crate) struct PackageVersion {
    /// The current version for the package
    pub(crate) version: Version,
    /// The package from which the version was derived (and the package that should be bumped).
    pub(crate) package: Package,
}

/// Bump the version of a single `package` using `rule`.
pub(super) fn bump_version(
    rule: &Rule,
    dry_run: bool,
    package: Package,
) -> Result<PackageVersion, StepError> {
    let versions = get_version(&package)?;
    let version = bump(versions, rule)?;
    let package = set_version(package, &version, dry_run)?;
    Ok(PackageVersion { version, package })
}

/// The implementation of [`crate::step::Step::BumpVersion`].
///
/// Bumps the version of every configured package using `rule`.
pub(crate) fn bump_version_and_update_state(
    run_type: RunType,
    rule: &Rule,
) -> Result<RunType, StepError> {
    let (mut dry_run_stdout, mut state) = match run_type {
        RunType::DryRun { state, stdout } => (Some(stdout), state),
        RunType::Real(state) => (None, state),
    };

    for package in state.packages.iter().cloned() {
        let PackageVersion { package, version } =
            bump_version(rule, dry_run_stdout.is_some(), package)?;
        if let Some(stdout) = dry_run_stdout.as_mut() {
            writeln!(
                stdout,
                "Would bump {name} to version {version}",
                name = package.name.as_deref().unwrap_or("package"),
                version = version
            )?;
        }
        state.releases.push(state::Release::Bumped {
            version,
            package_name: package.name.clone(),
        });
    }
    if let Some(stdout) = dry_run_stdout {
        Ok(RunType::DryRun { state, stdout })
    } else {
        Ok(RunType::Real(state))
    }
}

/// Get the current version of a package.
pub(crate) fn get_version(package: &Package) -> Result<CurrentVersions, StepError> {
    let version_from_files = package
        .versioned_files
        .iter()
        .map(|versioned_file| versioned_file.get_version(package.name.as_deref()))
        .map(|result| result.and_then(|version_string| Version::from_str(&version_string)))
        .reduce(|accumulator, version| match (version, accumulator) {
            (Ok(version), Ok(accumulator)) => {
                if version == accumulator {
                    Ok(accumulator)
                } else {
                    Err(StepError::InconsistentVersions(
                        version.to_string(),
                        accumulator.to_string(),
                    ))
                }
            }
            (_, Err(err)) | (Err(err), _) => Err(err),
        })
        .transpose()?;

    let mut current_versions = get_current_versions_from_tag(package.name.as_deref())?;

    if let Some(version_from_files) = version_from_files {
        if version_from_files.pre.is_none() {
            current_versions.replace_stable_if_newer(version_from_files);
        } else {
            current_versions.insert_prerelease(version_from_files);
        }
    }

    Ok(current_versions)
}

/// Consumes a [`PackageVersion`], writing it back to the file it came from. Returns the new version
/// that was written.
fn set_version(
    mut package: Package,
    version: &Version,
    dry_run: bool,
) -> Result<Package, StepError> {
    if dry_run {
        return Ok(package);
    }
    let mut paths = Vec::with_capacity(package.versioned_files.len());
    for versioned_file in &mut package.versioned_files {
        versioned_file.set_version(version)?;
        paths.push(&versioned_file.path);
    }
    add_files(&paths)?;
    Ok(package)
}

/// Apply a Rule to a [`PackageVersion`], incrementing & resetting the correct components.
///
/// ### Versions 0.x
///
/// Versions with major component 0 have special meaning in Semantic Versioning and therefore have
/// different behavior:
/// 1. [`Rule::Major`] will bump the minor component.
/// 2. [`Rule::Minor`] will bump the patch component.
fn bump(mut versions: CurrentVersions, rule: &Rule) -> Result<Version, StepError> {
    let mut stable = versions.stable.unwrap_or_default();
    let is_0 = stable.major == 0;
    match (rule, is_0) {
        (Rule::Major, false) => {
            stable.major += 1;
            stable.minor = 0;
            stable.patch = 0;
            stable.pre = None;
            Ok(stable)
        }
        (Rule::Minor, false) | (Rule::Major, true) => {
            stable.minor += 1;
            stable.patch = 0;
            stable.pre = None;
            Ok(stable)
        }
        (Rule::Patch, _) | (Rule::Minor, true) => {
            stable.patch += 1;
            stable.pre = None;
            Ok(stable)
        }
        (Rule::Release, _) => {
            let version = versions
                .prereleases
                .pop_last()
                .map(|(version, _pre)| version)
                .ok_or_else(|| {
                    StepError::InvalidPreReleaseVersion(
                        "No prerelease version found, but a Release rule was requested".to_string(),
                    )
                })?;
            Ok(version)
        }
        (Rule::Pre { label, stable_rule }, _) => {
            bump_pre(stable, &versions.prereleases, label, *stable_rule)
        }
    }
}

#[cfg(test)]
mod test_bump {
    use rstest::rstest;

    use super::*;

    #[test]
    fn major() {
        let stable = Version {
            major: 1,
            minor: 2,
            patch: 3,
            pre: None,
        };
        let version = bump(stable.into(), &Rule::Major).unwrap();

        assert_eq!(
            version,
            Version {
                major: 2,
                minor: 0,
                patch: 0,
                pre: None
            }
        );
    }

    #[test]
    fn major_0() {
        let stable = Version {
            major: 0,
            minor: 1,
            patch: 2,
            pre: None,
        };
        let version = bump(stable.into(), &Rule::Major).unwrap();

        assert_eq!(
            version,
            Version {
                major: 0,
                minor: 2,
                patch: 0,
                pre: None
            }
        );
    }

    #[test]
    fn major_unset() {
        let version = bump(CurrentVersions::default(), &Rule::Major).unwrap();

        assert_eq!(
            version,
            Version {
                major: 0,
                minor: 1,
                patch: 0,
                pre: None
            }
        );
    }

    #[rstest]
    #[case("1.2.4-rc.0")]
    #[case("1.3.0-rc.0")]
    #[case("2.0.0-rc.0")]
    fn major_after_pre(#[case] pre_version: &str) {
        let mut versions = CurrentVersions::from(Version {
            major: 1,
            minor: 2,
            patch: 3,
            pre: None,
        });
        versions.insert_prerelease(Version::from_str(pre_version).unwrap());
        let version = bump(versions, &Rule::Major).unwrap();

        assert_eq!(
            version,
            Version {
                major: 2,
                minor: 0,
                patch: 0,
                pre: None
            }
        );
    }

    #[test]
    fn minor() {
        let stable = Version {
            major: 1,
            minor: 2,
            patch: 3,
            pre: None,
        };
        let version = bump(stable.into(), &Rule::Minor).unwrap();

        assert_eq!(
            version,
            Version {
                major: 1,
                minor: 3,
                patch: 0,
                pre: None
            }
        );
    }

    #[test]
    fn minor_0() {
        let stable = Version {
            major: 0,
            minor: 1,
            patch: 2,
            pre: None,
        };
        let version = bump(stable.into(), &Rule::Minor).unwrap();

        assert_eq!(
            version,
            Version {
                major: 0,
                minor: 1,
                patch: 3,
                pre: None
            }
        );
    }

    #[test]
    fn minor_unset() {
        let version = bump(CurrentVersions::default(), &Rule::Minor).unwrap();

        assert_eq!(
            version,
            Version {
                major: 0,
                minor: 0,
                patch: 1,
                pre: None
            }
        );
    }

    #[rstest]
    #[case("1.2.4-rc.0")]
    #[case("1.3.0-rc.0")]
    fn minor_after_pre(#[case] pre_version: &str) {
        let mut versions = CurrentVersions::from(Version {
            major: 1,
            minor: 2,
            patch: 3,
            pre: None,
        });
        versions.insert_prerelease(Version::from_str(pre_version).unwrap());
        let version = bump(versions, &Rule::Minor).unwrap();

        assert_eq!(
            version,
            Version {
                major: 1,
                minor: 3,
                patch: 0,
                pre: None
            }
        );
    }

    #[test]
    fn patch() {
        let stable = Version {
            major: 1,
            minor: 2,
            patch: 3,
            pre: None,
        };
        let version = bump(stable.into(), &Rule::Patch).unwrap();

        assert_eq!(
            version,
            Version {
                major: 1,
                minor: 2,
                patch: 4,
                pre: None
            }
        );
    }

    #[test]
    fn patch_0() {
        let stable = Version {
            major: 0,
            minor: 1,
            patch: 0,
            pre: None,
        };
        let version = bump(stable.into(), &Rule::Patch).unwrap();

        assert_eq!(
            version,
            Version {
                major: 0,
                minor: 1,
                patch: 1,
                pre: None
            }
        );
    }

    #[test]
    fn patch_unset() {
        let version = bump(CurrentVersions::default(), &Rule::Patch).unwrap();

        assert_eq!(
            version,
            Version {
                major: 0,
                minor: 0,
                patch: 1,
                pre: None
            }
        );
    }

    #[test]
    fn patch_after_pre() {
        let mut versions = CurrentVersions::from(Version {
            major: 1,
            minor: 2,
            patch: 3,
            pre: None,
        });
        versions.insert_prerelease(Version::from_str("1.2.4-rc.0").unwrap());
        let version = bump(versions, &Rule::Patch).unwrap();

        assert_eq!(
            version,
            Version {
                major: 1,
                minor: 2,
                patch: 4,
                pre: None
            }
        );
    }

    #[test]
    fn pre() {
        let stable = Version {
            major: 1,
            minor: 2,
            patch: 3,
            pre: None,
        };
        let new = bump(
            stable.into(),
            &Rule::Pre {
                label: Label::from("rc"),
                stable_rule: ConventionalRule::Minor,
            },
        )
        .unwrap();

        assert_eq!(new, Version::from_str("1.3.0-rc.0").unwrap());
    }

    #[test]
    fn pre_after_same_pre() {
        let mut versions = CurrentVersions::from(Version {
            major: 1,
            minor: 2,
            patch: 3,
            pre: None,
        });
        versions.insert_prerelease(Version::from_str("1.3.0-rc.0").unwrap());
        versions.insert_prerelease(Version::from_str("1.2.4-rc.1").unwrap());
        versions.insert_prerelease(Version::from_str("2.0.0-rc.2").unwrap());
        let new = bump(
            versions,
            &Rule::Pre {
                label: Label::from("rc"),
                stable_rule: ConventionalRule::Minor,
            },
        )
        .unwrap();

        assert_eq!(new, Version::from_str("1.3.0-rc.1").unwrap());
    }

    #[test]
    fn pre_after_different_pre_version() {
        let mut versions = CurrentVersions::from(Version {
            major: 1,
            minor: 2,
            patch: 3,
            pre: None,
        });
        versions.insert_prerelease(Version::from_str("1.2.4-beta.1").unwrap());
        versions.insert_prerelease(Version::from_str("1.2.4-rc.0").unwrap());
        let new = bump(
            versions,
            &Rule::Pre {
                label: Label::from("beta"),
                stable_rule: ConventionalRule::Patch,
            },
        )
        .unwrap();

        assert_eq!(new, Version::from_str("1.2.4-beta.2").unwrap());
    }

    #[test]
    fn pre_after_different_pre_label() {
        let mut versions = CurrentVersions::from(Version {
            major: 1,
            minor: 2,
            patch: 3,
            pre: None,
        });
        versions.insert_prerelease(Version::from_str("1.3.0-beta.0").unwrap());
        let new = bump(
            versions,
            &Rule::Pre {
                label: Label::from("rc"),
                stable_rule: ConventionalRule::Minor,
            },
        )
        .unwrap();

        assert_eq!(new, Version::from_str("1.3.0-rc.0").unwrap());
    }

    #[test]
    fn release() {
        let mut versions = CurrentVersions::default();
        versions.insert_prerelease(Version::from_str("1.2.3-rc.0").unwrap());
        versions.insert_prerelease(Version::from_str("1.2.4-rc.1").unwrap());
        versions.insert_prerelease(Version::from_str("2.0.0-rc.2").unwrap());

        let version = bump(versions, &Rule::Release).unwrap();

        assert_eq!(
            version,
            Version {
                major: 2,
                minor: 0,
                patch: 0,
                pre: None
            }
        );
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
    stable: Version,
    prereleases: &Prereleases,
    label: &Label,
    stable_rule: ConventionalRule,
) -> Result<Version, StepError> {
    let stable_only = CurrentVersions {
        stable: Some(stable),
        ..Default::default()
    };
    let next_stable = bump(stable_only, &stable_rule.into())?;
    let prerelease = prereleases
        .get(&next_stable)
        .and_then(|pres| {
            pres.get(label).cloned().map(|mut pre| {
                pre.version += 1;
                pre
            })
        })
        .unwrap_or_else(|| Prerelease::new(label.clone(), 0));

    let mut next_prerelease = next_stable;
    next_prerelease.pre = Some(prerelease);
    Ok(next_prerelease)
}
