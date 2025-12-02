use std::{collections::BTreeMap, fmt::Debug, str::FromStr};

use tracing::debug;

use super::{
    Label, PreVersion, Prerelease, Rule, StableVersion, Version, prerelease_map::PrereleaseMap,
};
use crate::semver::rule::Stable;

/// It's not enough to just track one version for each package, we need:
/// - The latest stable version (if any)
/// - The last version of each type of pre-release following the latest stable version
///
/// So we might have 1.2.3, 1.2.4-rc.1, 1.3.0-beta.0, and 2.0.0-alpha.4
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackageVersions {
    stable: Option<StableVersion>,
    prereleases: Prereleases,
}

type Prereleases = BTreeMap<StableVersion, PrereleaseMap>;

impl PackageVersions {
    /// Get the (relevant) current versions from a slice of Git tags.
    ///
    /// Tags are expected to either be `v{version}` or `{prefix}/v{version}` (if supplied),
    ///
    /// ## Parameters
    /// - `prefix`: Only tag names starting with this string will be considered.
    /// - `all_tags`: All tags in the repository.
    pub fn from_tags<S: AsRef<str> + Debug>(prefix: Option<&str>, all_tags: &[S]) -> Self {
        let pattern = prefix
            .as_ref()
            .map_or_else(|| String::from("v"), |prefix| format!("{prefix}/v"));
        let mut tags = all_tags
            .iter()
            .filter(|tag| tag.as_ref().starts_with(&pattern))
            .peekable();

        if tags.peek().is_none() {
            debug!("No tags found starting with {pattern}");
        }

        let mut current_versions = Self::default();
        for tag in tags {
            let version_string = tag.as_ref().replacen(&pattern, "", 1);
            if let Ok(version) = Version::from_str(version_string.as_str()) {
                match version {
                    Version::Stable(stable) => {
                        current_versions.stable = Some(stable);
                        break; // Only prereleases newer than the last stable version are relevant
                    }
                    Version::Pre(_) => {
                        current_versions.update_version(version);
                    }
                }
            }
        }

        current_versions
    }

    /// Consumes `self` to produce the most recent version (determined by order of tags).
    #[must_use]
    pub fn into_latest(mut self) -> Option<Version> {
        self.prereleases
            .pop_last()
            .map(|(stable_component, pres)| {
                let pre_component = pres.into_last();
                Version::Pre(PreVersion {
                    stable_component,
                    pre_component,
                })
            })
            .or_else(|| self.stable.map(Version::Stable))
    }

    /// Replace or insert the version in the correct location if it's newer than the current
    /// equivalent version.
    /// If the version is a newer stable version, it will update `stable`
    /// and erase all pre-releases.
    /// If the version is a newer prerelease, it will overwrite the prerelease with
    /// the same stable component and label.
    pub(crate) fn update_version(&mut self, version: Version) {
        match version {
            Version::Stable(new) => {
                if self.stable.is_some_and(|it| it >= new) {
                    return;
                }
                self.stable = Some(new);
                self.prereleases.clear();
            }
            Version::Pre(PreVersion {
                stable_component,
                pre_component,
            }) => {
                let recorded_pre = self
                    .prereleases
                    .get(&stable_component)
                    .and_then(|pres| pres.get(&pre_component.label));
                if let Some(recorded_pre) = recorded_pre {
                    if recorded_pre >= &pre_component {
                        return;
                    }
                }
                if let Some(labels) = self.prereleases.get_mut(&stable_component) {
                    labels.insert(pre_component);
                } else {
                    self.prereleases
                        .insert(stable_component, PrereleaseMap::new(pre_component));
                }
            }
        }
    }

    /// Apply a Rule to a [`PackageVersion`], incrementing & resetting the correct components.
    ///
    /// # Versions 0.x
    ///
    /// Versions with major component 0 have special meaning in Semantic Versioning and therefore have
    /// different behavior:
    /// 1. [`Rule::Major`] will bump the minor component.
    /// 2. [`Stable(Minor)`] will bump the patch component.
    ///
    /// # Errors
    ///
    /// Can fail if trying to run [`Rule::Release`] when there is no pre-release.
    pub fn bump(&mut self, rule: Rule) -> Result<Version, PreReleaseNotFound> {
        match (rule, self.stable) {
            (Rule::Stable(rule), Some(stable)) => {
                let version: Version = bump_stable(stable, rule).into();
                self.update_version(version.clone());
                Ok(version)
            }
            (Rule::Stable(_), None) => {
                // Bumping the stable version, but there is no previous stable version.
                // So we use the last pre-release version as-is (assuming it was set manually)
                // _or_ 0.0.0 as the first version of the project.
                let version: Version = self
                    .prereleases
                    .pop_last()
                    .map(|(version, _pre)| version)
                    .unwrap_or_default()
                    .into();
                self.update_version(version.clone());
                Ok(version)
            }
            (Rule::Pre { label, stable_rule }, _) => Ok(self.bump_pre(label, stable_rule)),
            (Rule::Release, _) => {
                let version: Version = self
                    .prereleases
                    .pop_last()
                    .map(|(version, _pre)| version)
                    .ok_or(PreReleaseNotFound)?
                    .into();
                self.update_version(version.clone());
                Ok(version)
            }
        }
    }

    #[must_use]
    pub fn stable(&self) -> Option<StableVersion> {
        self.stable
    }

    /// Bumps the pre-release component of a [`Version`] after applying the `stable_rule`.
    fn bump_pre(&mut self, label: Label, stable_rule: Stable) -> Version {
        debug!("Pre-release label {label} selected. Determining next stable version...");
        let stable_component = if let Some(stable) = self.stable {
            bump_stable(stable, stable_rule)
        } else {
            self.prereleases
                .last_key_value()
                .map(|(stable, _)| *stable)
                .unwrap_or_default()
        };
        let pre_version = self
            .prereleases
            .get(&stable_component)
            .and_then(|pres| {
                pres.get(&label).map(|pre| {
                    debug!("Found existing pre-release version {pre}");
                    pre.version + 1
                })
            })
            .unwrap_or_default();
        let pre = Prerelease::new(label, pre_version);
        if pre_version == 0 {
            debug!("No existing pre-release version found; creating {pre}");
        }

        self.prereleases.clear();

        let version = Version::Pre(PreVersion {
            stable_component,
            pre_component: pre,
        });
        self.update_version(version.clone());
        version
    }
}

fn bump_stable(version: StableVersion, rule: Stable) -> StableVersion {
    let is_0 = version.major == 0;
    match (rule, is_0) {
        (Stable::Major, false) => {
            let new = version.increment_major();
            debug!("Using MAJOR rule to bump from {version} to {new}");
            new
        }
        (Stable::Minor, false) => {
            let new = version.increment_minor();
            debug!("Using MINOR rule to bump from {version} to {new}");
            new
        }
        (Stable::Major, true) => {
            let new = version.increment_minor();
            debug!(
                "Rule is MAJOR, but major component is 0. Bumping minor component from {version} to {new}"
            );
            new
        }
        (Stable::Minor, true) => {
            let new = version.increment_patch();
            debug!(
                "Rule is MINOR, but major component is 0. Bumping patch component from {version} to {new}"
            );
            new
        }
        (Stable::Patch, _) => {
            let new = version.increment_patch();
            debug!("Using PATCH rule to bump from {version} to {new}");
            new
        }
    }
}

// #[derive(Debug, thiserror::Error)]
// #[cfg_attr(feature = "miette", derive(Diagnostic))]
// #[error("Could not increment pre-release version {0}")]
// #[cfg_attr(
//     feature = "miette",
//     diagnostic(
//         code(semver::invalid_pre_release_version),
//         help(
//             "The pre-release component of a version must be in the format of `-<label>.N` \
//                     where <label> is a string and `N` is an integer"
//         ),
//         url("https://knope.tech/reference/concepts/semantic-versioning/#types-of-releases")
//     )
// )]
// pub(crate) struct InvalidPreReleaseVersion(String);

#[derive(Debug, thiserror::Error)]
#[error("No prerelease version found, but a Release rule was requested")]
pub struct PreReleaseNotFound;

impl From<StableVersion> for PackageVersions {
    fn from(version: StableVersion) -> Self {
        Self {
            stable: Some(version),
            prereleases: BTreeMap::new(),
        }
    }
}

impl From<Version> for PackageVersions {
    fn from(version: Version) -> Self {
        let mut new = Self::default();
        new.update_version(version);
        new
    }
}

#[cfg(test)]
mod test_from_tags {
    use std::str::FromStr;

    use pretty_assertions::assert_eq;

    use crate::semver::{PackageVersions, Prerelease, StableVersion, Version};
    #[test]
    fn collect_all_newer_pre_releases() {
        let tags = [
            "v2.0.0-alpha.0",
            "v1.3.0-beta.0",
            "v1.3.0-alpha.1",
            "v1.3.0-alpha.0",
            "v1.2.4-rc.0",
            "v1.2.3",
        ]
        .map(String::from);

        let versions = PackageVersions::from_tags(None, &tags);

        assert_eq!(
            versions.stable().unwrap(),
            StableVersion {
                major: 1,
                minor: 2,
                patch: 3
            }
        );

        assert_eq!(
            versions.clone().into_latest(),
            Version::from_str("2.0.0-alpha.0").ok()
        );
        assert_eq!(
            *versions
                .prereleases
                .get(&StableVersion {
                    major: 1,
                    minor: 3,
                    patch: 0
                })
                .unwrap()
                .get(&"alpha".into())
                .unwrap(),
            Prerelease::new("alpha".into(), 1)
        );

        assert_eq!(
            *versions
                .prereleases
                .get(&StableVersion {
                    major: 1,
                    minor: 3,
                    patch: 0
                })
                .unwrap()
                .get(&"beta".into())
                .unwrap(),
            Prerelease::new("beta".into(), 0)
        );
    }
}

#[cfg(test)]
mod test_bump {
    use std::str::FromStr;

    use super::*;
    use crate::semver::{Rule::*, StableRule::*};

    #[test]
    fn major() {
        let mut versions: PackageVersions = Version::new(1, 2, 3, None).into();
        versions.bump(Stable(Major)).unwrap();

        assert_eq!(versions.into_latest().unwrap(), Version::new(2, 0, 0, None));
    }

    #[test]
    fn major_0() {
        let mut versions = PackageVersions::from(Version::new(0, 1, 2, None));
        versions.bump(Stable(Major)).unwrap();

        assert_eq!(versions.into_latest().unwrap(), Version::new(0, 2, 0, None));
    }

    #[test]
    fn major_pre_only() {
        let mut versions = PackageVersions::from_tags(None, &["v1.0.0-rc.0"]);
        versions.bump(Stable(Major)).unwrap();

        assert_eq!(versions.into_latest().unwrap(), Version::new(1, 0, 0, None));
    }

    #[test]
    fn major_unset() {
        let mut versions = PackageVersions::default();
        versions.bump(Stable(Major)).unwrap();

        assert_eq!(versions.into_latest().unwrap(), Version::new(0, 0, 0, None));
    }

    #[test]
    fn major_after_pre() {
        for pre_version in ["1.2.4-rc.0", "1.3.0-rc.0", "2.0.0-rc.0"] {
            let mut versions = PackageVersions::from(Version::new(1, 2, 3, None));
            versions.update_version(Version::from_str(pre_version).unwrap());
            versions.bump(Stable(Major)).unwrap();

            assert_eq!(versions.into_latest().unwrap(), Version::new(2, 0, 0, None));
        }
    }

    #[test]
    fn minor() {
        let mut versions = PackageVersions::from(Version::new(1, 2, 3, None));
        versions.bump(Stable(Minor)).unwrap();

        assert_eq!(versions.into_latest().unwrap(), Version::new(1, 3, 0, None));
    }

    #[test]
    fn minor_0() {
        let mut versions = PackageVersions::from(Version::new(0, 1, 2, None));
        versions.bump(Stable(Minor)).unwrap();

        assert_eq!(versions.into_latest().unwrap(), Version::new(0, 1, 3, None));
    }

    #[test]
    fn minor_pre_only() {
        let mut versions = PackageVersions::from_tags(None, &["v1.0.0-rc.0"]);
        versions.bump(Stable(Minor)).unwrap();
        assert_eq!(versions.into_latest().unwrap(), Version::new(1, 0, 0, None));
    }

    #[test]
    fn minor_unset() {
        let mut versions = PackageVersions::default();
        versions.bump(Stable(Minor)).unwrap();

        assert_eq!(versions.into_latest().unwrap(), Version::new(0, 0, 0, None));
    }

    #[test]
    fn minor_after_pre() {
        for pre_version in ["1.2.4-rc.0", "1.3.0-rc.0"] {
            let mut versions = PackageVersions::from(Version::new(1, 2, 3, None));
            versions.update_version(Version::from_str(pre_version).unwrap());
            versions.bump(Stable(Minor)).unwrap();

            assert_eq!(versions.into_latest().unwrap(), Version::new(1, 3, 0, None));
        }
    }

    #[test]
    fn patch() {
        let mut versions = PackageVersions::from(Version::new(1, 2, 3, None));
        versions.bump(Stable(Patch)).unwrap();

        assert_eq!(versions.into_latest().unwrap(), Version::new(1, 2, 4, None));
    }

    #[test]
    fn patch_0() {
        let mut versions = PackageVersions::from(Version::new(0, 1, 0, None));
        versions.bump(Stable(Patch)).unwrap();

        assert_eq!(versions.into_latest().unwrap(), Version::new(0, 1, 1, None));
    }

    #[test]
    fn patch_pre_only() {
        let mut versions = PackageVersions::from_tags(None, &["v1.0.0-rc.0"]);
        versions.bump(Stable(Patch)).unwrap();
        assert_eq!(versions.into_latest().unwrap(), Version::new(1, 0, 0, None));
    }

    #[test]
    fn patch_unset() {
        let mut versions = PackageVersions::default();
        versions.bump(Stable(Patch)).unwrap();

        assert_eq!(versions.into_latest().unwrap(), Version::new(0, 0, 0, None));
    }

    #[test]
    fn patch_after_pre() {
        let mut versions = PackageVersions::from(Version::new(1, 2, 3, None));
        versions.update_version(Version::from_str("1.2.4-rc.0").unwrap());
        versions.bump(Stable(Patch)).unwrap();

        assert_eq!(versions.into_latest().unwrap(), Version::new(1, 2, 4, None));
    }

    #[test]
    fn pre() {
        let mut versions = PackageVersions::from(Version::new(1, 2, 3, None));
        versions
            .bump(Rule::Pre {
                label: Label::from("rc"),
                stable_rule: Minor,
            })
            .unwrap();

        assert_eq!(versions.into_latest(), Version::from_str("1.3.0-rc.0").ok());
    }

    #[test]
    fn pre_after_same_pre() {
        let mut versions = PackageVersions::from(Version::new(1, 2, 3, None));
        versions.update_version(Version::from_str("1.3.0-rc.0").unwrap());
        versions.update_version(Version::from_str("1.2.4-rc.1").unwrap());
        versions.update_version(Version::from_str("2.0.0-rc.2").unwrap());
        versions
            .bump(Rule::Pre {
                label: Label::from("rc"),
                stable_rule: Minor,
            })
            .unwrap();

        assert_eq!(versions.into_latest(), Version::from_str("1.3.0-rc.1").ok());
    }

    #[test]
    fn pre_without_stable() {
        let mut versions = PackageVersions::default();
        versions.update_version(Version::from_str("1.3.0-rc.0").unwrap());
        versions.update_version(Version::from_str("1.2.4-rc.1").unwrap());
        versions.update_version(Version::from_str("2.0.0-rc.2").unwrap());
        versions
            .bump(Rule::Pre {
                label: Label::from("rc"),
                stable_rule: Minor,
            })
            .unwrap();

        assert_eq!(versions.into_latest(), Version::from_str("2.0.0-rc.3").ok());
    }

    #[test]
    fn pre_after_different_pre_version() {
        let mut versions = PackageVersions::from(Version::new(1, 2, 3, None));
        versions.update_version(Version::from_str("1.2.4-beta.1").unwrap());
        versions.update_version(Version::from_str("1.2.4-rc.0").unwrap());
        versions
            .bump(Rule::Pre {
                label: Label::from("beta"),
                stable_rule: Patch,
            })
            .unwrap();

        assert_eq!(
            versions.into_latest(),
            Version::from_str("1.2.4-beta.2").ok()
        );
    }

    #[test]
    fn pre_after_different_pre_label() {
        let mut versions = PackageVersions::from(Version::new(1, 2, 3, None));
        versions.update_version(Version::from_str("1.3.0-beta.0").unwrap());
        versions
            .bump(Rule::Pre {
                label: Label::from("rc"),
                stable_rule: Minor,
            })
            .unwrap();

        assert_eq!(versions.into_latest(), Version::from_str("1.3.0-rc.0").ok());
    }

    #[test]
    fn release() {
        let mut versions = PackageVersions::default();
        versions.update_version(Version::from_str("1.2.3-rc.0").unwrap());
        versions.update_version(Version::from_str("1.2.4-rc.1").unwrap());
        versions.update_version(Version::from_str("2.0.0-rc.2").unwrap());

        versions.bump(Rule::Release).unwrap();

        assert_eq!(versions.into_latest().unwrap(), Version::new(2, 0, 0, None));
    }
}
