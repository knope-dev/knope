use std::{collections::BTreeMap, fmt::Debug, str::FromStr};

use tracing::debug;

use super::{
    prerelease_map::PrereleaseMap, Label, PreVersion, Prerelease, Rule, StableVersion, Version,
};
use crate::semver::rule::Stable;

/// It's not enough to just track one version for each package, we need:
/// - The latest stable version (if any)
/// - The last version of each type of pre-release following the latest stable version
///
/// So we might have 1.2.3, 1.2.4-rc.1, 1.3.0-beta.0, and 2.0.0-alpha.4
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackageVersions {
    stable: StableVersion,
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
            debug!("No tags found matching pattern {pattern}");
        }

        let mut current_versions = Self::default();
        for tag in tags {
            let version_string = tag.as_ref().replace(&pattern, "");
            if let Ok(version) = Version::from_str(version_string.as_str()) {
                match version {
                    Version::Stable(stable) => {
                        current_versions.stable = stable;
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
    pub fn into_latest(mut self) -> Version {
        self.prereleases.pop_last().map_or(
            Version::Stable(self.stable),
            |(stable_component, pres)| {
                let pre_component = pres.into_last();
                Version::Pre(PreVersion {
                    stable_component,
                    pre_component,
                })
            },
        )
    }

    /// Replace or insert the version in the correct location if it's newer than the current
    /// equivalent version.
    /// If the version is a newer stable version, it will update `stable`
    /// and erase all pre-releases.
    /// If the version is a newer prerelease, it will overwrite the prerelease with
    /// the same stable component and label.
    pub fn update_version(&mut self, version: Version) {
        match version {
            Version::Stable(new) => {
                if self.stable >= new {
                    return;
                }
                self.stable = new;
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
    /// 2. [`Rule::Minor`] will bump the patch component.
    ///
    /// # Errors
    ///
    /// Can fail if trying to run [`Rule::Release`] when there is no pre-release.
    pub fn bump(&mut self, rule: Rule) -> Result<(), PreReleaseNotFound> {
        match rule {
            Rule::Major => self.update_version(bump_stable(self.stable, Stable::Major).into()),
            Rule::Minor => self.update_version(bump_stable(self.stable, Stable::Minor).into()),
            Rule::Patch => self.update_version(bump_stable(self.stable, Stable::Patch).into()),
            Rule::Release => {
                let version = self
                    .prereleases
                    .pop_last()
                    .map(|(version, _pre)| version)
                    .ok_or(PreReleaseNotFound)?
                    .into();
                self.update_version(version);
            }
            Rule::Pre { label, stable_rule } => self.bump_pre(label, stable_rule),
        }
        Ok(())
    }

    #[must_use]
    pub fn stable(&self) -> StableVersion {
        self.stable
    }

    /// Bumps the pre-release component of a [`Version`] after applying the `stable_rule`.
    ///
    /// # Errors
    ///
    /// Can fail if there's an existing pre-release component that can't be incremented.
    fn bump_pre(&mut self, label: Label, stable_rule: Stable) {
        debug!("Pre-release label {label} selected. Determining next stable version...");
        let stable_component = bump_stable(self.stable, stable_rule);
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

        self.update_version(Version::Pre(PreVersion {
            stable_component,
            pre_component: pre,
        }));
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
            debug!("Rule is MAJOR, but major component is 0. Bumping minor component from {version} to {new}");
            new
        }
        (Stable::Minor, true) => {
            let new = version.increment_patch();
            debug!("Rule is MINOR, but major component is 0. Bumping patch component from {version} to {new}");
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
            stable: version,
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
#[allow(clippy::unwrap_used)]
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
            versions.stable(),
            StableVersion {
                major: 1,
                minor: 2,
                patch: 3
            }
        );

        assert_eq!(
            versions.clone().into_latest(),
            Version::from_str("2.0.0-alpha.0").unwrap()
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
#[allow(clippy::unwrap_used)]
mod test_bump {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn major() {
        let mut versions: PackageVersions = Version::new(1, 2, 3, None).into();
        versions.bump(Rule::Major).unwrap();

        assert_eq!(versions.into_latest(), Version::new(2, 0, 0, None));
    }

    #[test]
    fn major_0() {
        let mut versions = PackageVersions::from(Version::new(0, 1, 2, None));
        versions.bump(Rule::Major).unwrap();

        assert_eq!(versions.into_latest(), Version::new(0, 2, 0, None));
    }

    #[test]
    fn major_unset() {
        let mut versions = PackageVersions::default();
        versions.bump(Rule::Major).unwrap();

        assert_eq!(versions.into_latest(), Version::new(0, 1, 0, None));
    }

    #[test]
    fn major_after_pre() {
        for pre_version in ["1.2.4-rc.0", "1.3.0-rc.0", "2.0.0-rc.0"] {
            let mut versions = PackageVersions::from(Version::new(1, 2, 3, None));
            versions.update_version(Version::from_str(pre_version).unwrap());
            versions.bump(Rule::Major).unwrap();

            assert_eq!(versions.into_latest(), Version::new(2, 0, 0, None));
        }
    }

    #[test]
    fn minor() {
        let mut versions = PackageVersions::from(Version::new(1, 2, 3, None));
        versions.bump(Rule::Minor).unwrap();

        assert_eq!(versions.into_latest(), Version::new(1, 3, 0, None));
    }

    #[test]
    fn minor_0() {
        let mut versions = PackageVersions::from(Version::new(0, 1, 2, None));
        versions.bump(Rule::Minor).unwrap();

        assert_eq!(versions.into_latest(), Version::new(0, 1, 3, None));
    }

    #[test]
    fn minor_unset() {
        let mut versions = PackageVersions::default();
        versions.bump(Rule::Minor).unwrap();

        assert_eq!(versions.into_latest(), Version::new(0, 0, 1, None));
    }

    #[test]
    fn minor_after_pre() {
        for pre_version in ["1.2.4-rc.0", "1.3.0-rc.0"] {
            let mut versions = PackageVersions::from(Version::new(1, 2, 3, None));
            versions.update_version(Version::from_str(pre_version).unwrap());
            versions.bump(Rule::Minor).unwrap();

            assert_eq!(versions.into_latest(), Version::new(1, 3, 0, None));
        }
    }

    #[test]
    fn patch() {
        let mut versions = PackageVersions::from(Version::new(1, 2, 3, None));
        versions.bump(Rule::Patch).unwrap();

        assert_eq!(versions.into_latest(), Version::new(1, 2, 4, None));
    }

    #[test]
    fn patch_0() {
        let mut versions = PackageVersions::from(Version::new(0, 1, 0, None));
        versions.bump(Rule::Patch).unwrap();

        assert_eq!(versions.into_latest(), Version::new(0, 1, 1, None));
    }

    #[test]
    fn patch_unset() {
        let mut versions = PackageVersions::default();
        versions.bump(Rule::Patch).unwrap();

        assert_eq!(versions.into_latest(), Version::new(0, 0, 1, None));
    }

    #[test]
    fn patch_after_pre() {
        let mut versions = PackageVersions::from(Version::new(1, 2, 3, None));
        versions.update_version(Version::from_str("1.2.4-rc.0").unwrap());
        versions.bump(Rule::Patch).unwrap();

        assert_eq!(versions.into_latest(), Version::new(1, 2, 4, None));
    }

    #[test]
    fn pre() {
        let mut versions = PackageVersions::from(Version::new(1, 2, 3, None));
        versions
            .bump(Rule::Pre {
                label: Label::from("rc"),
                stable_rule: Stable::Minor,
            })
            .unwrap();

        assert_eq!(
            versions.into_latest(),
            Version::from_str("1.3.0-rc.0").unwrap()
        );
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
                stable_rule: Stable::Minor,
            })
            .unwrap();

        assert_eq!(
            versions.into_latest(),
            Version::from_str("1.3.0-rc.1").unwrap()
        );
    }

    #[test]
    fn pre_after_different_pre_version() {
        let mut versions = PackageVersions::from(Version::new(1, 2, 3, None));
        versions.update_version(Version::from_str("1.2.4-beta.1").unwrap());
        versions.update_version(Version::from_str("1.2.4-rc.0").unwrap());
        versions
            .bump(Rule::Pre {
                label: Label::from("beta"),
                stable_rule: Stable::Patch,
            })
            .unwrap();

        assert_eq!(
            versions.into_latest(),
            Version::from_str("1.2.4-beta.2").unwrap()
        );
    }

    #[test]
    fn pre_after_different_pre_label() {
        let mut versions = PackageVersions::from(Version::new(1, 2, 3, None));
        versions.update_version(Version::from_str("1.3.0-beta.0").unwrap());
        versions
            .bump(Rule::Pre {
                label: Label::from("rc"),
                stable_rule: Stable::Minor,
            })
            .unwrap();

        assert_eq!(
            versions.into_latest(),
            Version::from_str("1.3.0-rc.0").unwrap()
        );
    }

    #[test]
    fn release() {
        let mut versions = PackageVersions::default();
        versions.update_version(Version::from_str("1.2.3-rc.0").unwrap());
        versions.update_version(Version::from_str("1.2.4-rc.1").unwrap());
        versions.update_version(Version::from_str("2.0.0-rc.2").unwrap());

        versions.bump(Rule::Release).unwrap();

        assert_eq!(versions.into_latest(), Version::new(2, 0, 0, None));
    }
}
