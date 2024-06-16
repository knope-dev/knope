use knope_versioning::changes::{conventional_commit::changes_from_commit_messages, Change};
use miette::Diagnostic;

use super::Package;
use crate::{
    integrations::git::{self, get_commit_messages_after_tag, get_current_versions_from_tags},
    step::releases::tag_name,
    workflow::Verbose,
};

fn get_conventional_commits_after_last_stable_version(
    package: &Package,
    verbose: Verbose,
    all_tags: &[String],
) -> Result<Vec<Change>, Error> {
    if let Verbose::Yes = verbose {
        println!(
            "Getting conventional commits since last release of package {}",
            package.name.as_deref().unwrap_or_default()
        );
        if let Some(scopes) = package.versioning.scopes.as_ref() {
            println!("Only checking commits with scopes: {scopes:?}");
        }
    }
    let target_version =
        get_current_versions_from_tags(package.name.as_deref(), verbose, all_tags).stable;
    let tag = target_version.map(|version| tag_name(&version.into(), &package.name));
    let commit_messages = get_commit_messages_after_tag(tag, verbose).map_err(git::Error::from)?;
    Ok(changes_from_commit_messages(
        &commit_messages,
        &package.versioning,
    ))
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(#[from] git::Error),
}

pub(crate) fn add_releases_from_conventional_commits(
    packages: Vec<Package>,
    tags: &[String],
    verbose: Verbose,
) -> Result<Vec<Package>, Error> {
    packages
        .into_iter()
        .map(|package| add_release_for_package(package, tags, verbose))
        .collect()
}

fn add_release_for_package(
    mut package: Package,
    tags: &[String],
    verbose: Verbose,
) -> Result<Package, Error> {
    get_conventional_commits_after_last_stable_version(&package, verbose, tags).map(|commits| {
        if !commits.is_empty() {
            package.pending_changes = commits;
        }
        package
    })
}
