use std::fmt::Display;

use knope_versioning::{
    changes::ChangeType, Label, PreVersion, Prerelease, StableVersion, Version,
};
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::{package::Package, CurrentVersions, Prereleases};
use crate::{
    fs,
    integrations::git,
    state::State,
    step::releases::versioned_file::{VersionFromSource, VersionSource},
    RunType,
};

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
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ConventionalRule {
    Major,
    Minor,
    #[default]
    Patch,
}

impl Display for ConventionalRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConventionalRule::Major => write!(f, "MAJOR"),
            ConventionalRule::Minor => write!(f, "MINOR"),
            ConventionalRule::Patch => write!(f, "PATCH"),
        }
    }
}

impl Ord for ConventionalRule {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Major, Self::Major)
            | (Self::Minor, Self::Minor)
            | (Self::Patch, Self::Patch) => std::cmp::Ordering::Equal,
            (Self::Major, _) | (_, Self::Patch) => std::cmp::Ordering::Greater,
            (_, Self::Major) | (Self::Patch, _) => std::cmp::Ordering::Less,
        }
    }
}

impl PartialOrd for ConventionalRule {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl From<&ChangeType> for ConventionalRule {
    fn from(value: &ChangeType) -> Self {
        match value {
            ChangeType::Feature => Self::Minor,
            ChangeType::Breaking => Self::Major,
            ChangeType::Custom(_) | ChangeType::Fix => Self::Patch,
        }
    }
}

/// The implementation of [`crate::step::Step::BumpVersion`].
///
/// Bumps the version of every configured package using `rule`.
pub(crate) fn bump_version_and_update_state(
    state: RunType<State>,
    rule: &Rule,
) -> Result<RunType<State>, Error> {
    let (mut state, dry_run) = match state {
        RunType::DryRun(state) => (state, true),
        RunType::Real(state) => (state, false),
    };

    state.packages = state
        .packages
        .into_iter()
        .map(|mut package| {
            let current = package.get_version(&state.all_git_tags).clone();
            let version = if let Some(version) = package.override_version.clone() {
                VersionFromSource {
                    version,
                    source: VersionSource::OverrideVersion,
                }
            } else {
                let version = bump(current.clone(), rule)?;
                VersionFromSource {
                    version,
                    source: VersionSource::Calculated,
                }
            };
            package
                .write_version(current, version, dry_run)
                .map_err(Error::from)
        })
        .collect::<Result<Vec<Package>, Error>>()?;
    if dry_run {
        Ok(RunType::DryRun(state))
    } else {
        Ok(RunType::Real(state))
    }
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum UpdatePackageVersionError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    KnopeVersioning(#[from] knope_versioning::SetError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    InvalidPreReleaseVersion(#[from] InvalidPreReleaseVersion),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(#[from] git::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    UpdatePackageVersion(#[from] UpdatePackageVersionError),
}

#[derive(Debug, Diagnostic, thiserror::Error)]
#[error("Could not increment pre-release version {0}")]
#[diagnostic(
    code(semver::invalid_pre_release_version),
    help(
        "The pre-release component of a version must be in the format of `-<label>.N` \
                    where <label> is a string and `N` is an integer"
    ),
    url("https://knope.tech/reference/concepts/semantic-versioning/#types-of-releases")
)]
pub(crate) struct InvalidPreReleaseVersion(String);

/// Apply a Rule to a [`PackageVersion`], incrementing & resetting the correct components.
///
/// ### Versions 0.x
///
/// Versions with major component 0 have special meaning in Semantic Versioning and therefore have
/// different behavior:
/// 1. [`Rule::Major`] will bump the minor component.
/// 2. [`Rule::Minor`] will bump the patch component.
pub(crate) fn bump(
    mut versions: CurrentVersions,
    rule: &Rule,
) -> Result<Version, InvalidPreReleaseVersion> {
    let stable = versions.stable.unwrap_or_default();
    let is_0 = stable.major == 0;
    match (rule, is_0) {
        (Rule::Major, false) => {
            let new_stable = stable.increment_major();
            debug!("Using MAJOR rule to bump from {stable} to {new_stable}");
            Ok(Version::Stable(new_stable))
        }
        (Rule::Minor, false) => {
            let new_stable = stable.increment_minor();
            debug!("Using MINOR rule to bump from {stable} to {new_stable}");
            Ok(Version::Stable(new_stable))
        }
        (Rule::Major, true) => {
            let new_stable = stable.increment_minor();
            debug!("Rule is MAJOR, but major component is 0. Bumping minor component from {stable} to {new_stable}");
            Ok(Version::Stable(new_stable))
        }
        (Rule::Minor, true) => {
            let new_stable = stable.increment_patch();
            debug!("Rule is MINOR, but major component is 0. Bumping patch component from {stable} to {new_stable}");
            Ok(Version::Stable(new_stable))
        }
        (Rule::Patch, _) => {
            let new_stable = stable.increment_patch();
            debug!("Using PATCH rule to bump from {stable} to {new_stable}");
            Ok(Version::Stable(new_stable))
        }
        (Rule::Release, _) => {
            let version = versions
                .prereleases
                .pop_last()
                .map(|(version, _pre)| version)
                .ok_or_else(|| {
                    InvalidPreReleaseVersion(
                        "No prerelease version found, but a Release rule was requested".to_string(),
                    )
                })?;
            Ok(Version::Stable(version))
        }
        (Rule::Pre { label, stable_rule }, _) => {
            bump_pre(stable, &versions.prereleases, label, *stable_rule)
        }
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
    stable: StableVersion,
    prereleases: &Prereleases,
    label: &Label,
    stable_rule: ConventionalRule,
) -> Result<Version, InvalidPreReleaseVersion> {
    debug!("Pre-release label {label} selected. Determining next stable version...");
    let stable_component = bump(stable.into(), &stable_rule.into())?.stable_component();
    let pre_component = prereleases
        .get(&stable_component)
        .and_then(|pres| {
            pres.get(label).cloned().map(|mut pre| {
                debug!("Found existing pre-release version {pre}");
                pre.version += 1;
                pre
            })
        })
        .unwrap_or_else(|| {
            let pre = Prerelease::new(label.clone(), 0);
            debug!("No existing pre-release version found; creating {pre}");
            pre
        });

    Ok(Version::Pre(PreVersion {
        stable_component,
        pre_component,
    }))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test_bump {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn major() {
        let stable = Version::new(1, 2, 3, None);
        let version = bump(stable.into(), &Rule::Major).unwrap();

        assert_eq!(version, Version::new(2, 0, 0, None));
    }

    #[test]
    fn major_0() {
        let stable = Version::new(0, 1, 2, None);
        let version = bump(stable.into(), &Rule::Major).unwrap();

        assert_eq!(version, Version::new(0, 2, 0, None));
    }

    #[test]
    fn major_unset() {
        let version = bump(CurrentVersions::default(), &Rule::Major).unwrap();

        assert_eq!(version, Version::new(0, 1, 0, None));
    }

    #[test]
    fn major_after_pre() {
        for pre_version in ["1.2.4-rc.0", "1.3.0-rc.0", "2.0.0-rc.0"] {
            let mut versions = CurrentVersions::from(Version::new(1, 2, 3, None));
            versions.update_version(Version::from_str(pre_version).unwrap());
            let version = bump(versions, &Rule::Major).unwrap();

            assert_eq!(version, Version::new(2, 0, 0, None));
        }
    }

    #[test]
    fn minor() {
        let stable = Version::new(1, 2, 3, None);
        let version = bump(stable.into(), &Rule::Minor).unwrap();

        assert_eq!(version, Version::new(1, 3, 0, None));
    }

    #[test]
    fn minor_0() {
        let stable = Version::new(0, 1, 2, None);
        let version = bump(stable.into(), &Rule::Minor).unwrap();

        assert_eq!(version, Version::new(0, 1, 3, None));
    }

    #[test]
    fn minor_unset() {
        let version = bump(CurrentVersions::default(), &Rule::Minor).unwrap();

        assert_eq!(version, Version::new(0, 0, 1, None));
    }

    #[test]
    fn minor_after_pre() {
        for pre_version in ["1.2.4-rc.0", "1.3.0-rc.0"] {
            let mut versions = CurrentVersions::from(Version::new(1, 2, 3, None));
            versions.update_version(Version::from_str(pre_version).unwrap());
            let version = bump(versions, &Rule::Minor).unwrap();

            assert_eq!(version, Version::new(1, 3, 0, None));
        }
    }

    #[test]
    fn patch() {
        let stable = Version::new(1, 2, 3, None);
        let version = bump(stable.into(), &Rule::Patch).unwrap();

        assert_eq!(version, Version::new(1, 2, 4, None));
    }

    #[test]
    fn patch_0() {
        let stable = Version::new(0, 1, 0, None);
        let version = bump(stable.into(), &Rule::Patch).unwrap();

        assert_eq!(version, Version::new(0, 1, 1, None));
    }

    #[test]
    fn patch_unset() {
        let version = bump(CurrentVersions::default(), &Rule::Patch).unwrap();

        assert_eq!(version, Version::new(0, 0, 1, None));
    }

    #[test]
    fn patch_after_pre() {
        let mut versions = CurrentVersions::from(Version::new(1, 2, 3, None));
        versions.update_version(Version::from_str("1.2.4-rc.0").unwrap());
        let version = bump(versions, &Rule::Patch).unwrap();

        assert_eq!(version, Version::new(1, 2, 4, None));
    }

    #[test]
    fn pre() {
        let stable = Version::new(1, 2, 3, None);
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
        let mut versions = CurrentVersions::from(Version::new(1, 2, 3, None));
        versions.update_version(Version::from_str("1.3.0-rc.0").unwrap());
        versions.update_version(Version::from_str("1.2.4-rc.1").unwrap());
        versions.update_version(Version::from_str("2.0.0-rc.2").unwrap());
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
        let mut versions = CurrentVersions::from(Version::new(1, 2, 3, None));
        versions.update_version(Version::from_str("1.2.4-beta.1").unwrap());
        versions.update_version(Version::from_str("1.2.4-rc.0").unwrap());
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
        let mut versions = CurrentVersions::from(Version::new(1, 2, 3, None));
        versions.update_version(Version::from_str("1.3.0-beta.0").unwrap());
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
        versions.update_version(Version::from_str("1.2.3-rc.0").unwrap());
        versions.update_version(Version::from_str("1.2.4-rc.1").unwrap());
        versions.update_version(Version::from_str("2.0.0-rc.2").unwrap());

        let version = bump(versions, &Rule::Release).unwrap();

        assert_eq!(version, Version::new(2, 0, 0, None));
    }
}
