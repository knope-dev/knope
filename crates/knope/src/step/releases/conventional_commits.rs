use knope_versioning::{package, ReleaseTag};

use crate::{
    integrations::git::{self, get_commit_messages_after_tag, get_current_versions_from_tags},
    workflow::Verbose,
};

pub(crate) fn get_conventional_commits_after_last_stable_version(
    package_name: &package::Name,
    scopes: Option<&Vec<String>>,
    verbose: Verbose,
    all_tags: &[String],
) -> Result<Vec<String>, git::Error> {
    if let Verbose::Yes = verbose {
        println!(
            "Getting conventional commits since last release of package {}",
            package_name.as_custom().unwrap_or_default()
        );
        if let Some(scopes) = scopes {
            println!("Only checking commits with scopes: {scopes:?}");
        }
    }
    let target_version =
        get_current_versions_from_tags(package_name.as_custom(), verbose, all_tags).stable;
    let tag = target_version.map(|version| ReleaseTag::new(&version.into(), package_name));

    get_commit_messages_after_tag(tag.as_ref().map(ReleaseTag::as_str), verbose)
        .map_err(git::Error::from)
}
