use knope_versioning::{package, ReleaseTag};
use tracing::debug;

use crate::integrations::git::{
    self, get_commit_messages_after_tag, get_current_versions_from_tags,
};

pub(crate) fn get_conventional_commits_after_last_stable_version(
    package_name: &package::Name,
    scopes: Option<&Vec<String>>,
    all_tags: &[String],
) -> Result<Vec<String>, git::Error> {
    debug!(
        "Getting conventional commits since last release of package {}",
        package_name.as_custom().unwrap_or_default()
    );
    if let Some(scopes) = scopes {
        debug!("Only checking commits with scopes: {scopes:?}");
    }
    let target_version = get_current_versions_from_tags(package_name.as_custom(), all_tags).stable;
    let tag = target_version.map(|version| ReleaseTag::new(&version.into(), package_name));

    get_commit_messages_after_tag(tag.as_ref().map(ReleaseTag::as_str)).map_err(git::Error::from)
}
