use knope_versioning::{
    changelog::Sections,
    changes::{conventional_commit::changes_from_commit_messages, Change},
};

use super::PackageName;
use crate::{
    integrations::git::{self, get_commit_messages_after_tag, get_current_versions_from_tags},
    step::releases::tag_name,
    workflow::Verbose,
};

pub(crate) fn get_conventional_commits_after_last_stable_version(
    package_name: &Option<PackageName>,
    scopes: Option<&Vec<String>>,
    changelog_sections: &Sections,
    verbose: Verbose,
    all_tags: &[String],
) -> Result<Vec<Change>, git::Error> {
    if let Verbose::Yes = verbose {
        println!(
            "Getting conventional commits since last release of package {}",
            package_name.as_deref().unwrap_or_default()
        );
        if let Some(scopes) = scopes {
            println!("Only checking commits with scopes: {scopes:?}");
        }
    }
    let target_version =
        get_current_versions_from_tags(package_name.as_deref(), verbose, all_tags).stable;
    let tag = target_version.map(|version| tag_name(&version.into(), package_name));
    let commit_messages = get_commit_messages_after_tag(tag, verbose).map_err(git::Error::from)?;
    Ok(changes_from_commit_messages(
        &commit_messages,
        scopes,
        changelog_sections,
    ))
}
