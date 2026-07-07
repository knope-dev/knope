use knope_versioning::{
    ReleaseTag, changes::conventional_commit::Commit, package, semver::PackageVersions,
};
use tracing::debug;

use crate::integrations::git::{self, get_commit_messages_after_tag};

/// Collect conventional commits from Git for a single package.
///
/// In stable mode (`prerelease_mode = false`) we look back to the most recent **stable**
/// release tag, since prereleases are not user-facing releases.
///
/// In prerelease mode (`prerelease_mode = true`) we look back to the most recent tag of
/// any kind — including prereleases. This lets iterative prerelease workflows (e.g.,
/// alpha.1 → alpha.2) consider only the commits added since the previous prerelease,
/// rather than re-summarizing every commit since the last stable.
pub(crate) fn get_conventional_commits_after_last_release(
    package_name: &package::Name,
    all_tags: &[String],
    prerelease_mode: bool,
) -> Result<Vec<Commit>, git::Error> {
    debug!(
        "Getting conventional commits since last release of package {}",
        package_name.as_custom().unwrap_or_default()
    );
    let versions = PackageVersions::from_tags(package_name.as_custom(), all_tags);
    let tag = if prerelease_mode {
        versions
            .latest()
            .map(|target_version| ReleaseTag::new(&target_version, package_name))
    } else {
        versions
            .stable()
            .map(|target_version| ReleaseTag::new(&target_version.into(), package_name))
    };

    get_commit_messages_after_tag(tag.as_ref().map(ReleaseTag::as_str))
}
