use knope_versioning::{
    ReleaseTag, changes::conventional_commit::Commit, package, semver::PackageVersions,
};
use tracing::debug;

use crate::integrations::git::{self, get_commit_messages_after_tag};

pub(crate) fn get_conventional_commits_after_last_stable_version(
    package_name: &package::Name,
    all_tags: &[String],
) -> Result<Vec<Commit>, git::Error> {
    debug!(
        "Getting conventional commits since last release of package {}",
        package_name.as_custom().unwrap_or_default()
    );
    let target_version = PackageVersions::from_tags(package_name.as_custom(), all_tags).stable();
    let tag = ReleaseTag::new(&target_version.into(), package_name);

    get_commit_messages_after_tag(tag.as_str())
}
