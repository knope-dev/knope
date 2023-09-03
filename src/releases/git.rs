use std::{env::current_dir, io::Write, str::FromStr};

use gix::{object::Kind, open, refs::transaction::PreviousValue};
use miette::Diagnostic;
use thiserror::Error;

use crate::{
    releases::{semver::Version, CurrentVersions, PackageName},
    step::StepError,
};

pub(crate) fn tag_name(version: &Version, package_name: Option<&PackageName>) -> String {
    let prefix = package_name
        .as_ref()
        .map_or_else(|| "v".to_string(), |name| format!("{name}/v"));
    format!("{prefix}{version}")
}

pub(crate) fn release(
    dry_run_stdout: &mut Option<Box<dyn Write>>,
    version: &Version,
    package_name: Option<&PackageName>,
) -> Result<(), StepError> {
    let tag = tag_name(version, package_name);

    create_tag(dry_run_stdout, tag)?;

    Ok(())
}

pub(crate) fn create_tag(dry_run: &mut Option<Box<dyn Write>>, name: String) -> Result<(), Error> {
    if let Some(stdout) = dry_run {
        return writeln!(stdout, "Would create Git tag {name}").map_err(Error::Stdout);
    }
    let repo = open(current_dir().map_err(Error::CurrentDirectory)?)
        .map_err(|err| Error::OpenGitRepo(Box::new(err)))?;
    let head = repo.head_commit()?;
    repo.tag(
        name,
        head.id,
        Kind::Commit,
        repo.committer()
            .transpose()
            .map_err(|_| Error::NoCommitter)?,
        "",
        PreviousValue::Any,
    )?;
    Ok(())
}

pub(crate) fn get_current_versions_from_tag(
    prefix: Option<&str>,
) -> Result<CurrentVersions, Error> {
    let repo = open(current_dir().map_err(Error::CurrentDirectory)?)
        .map_err(|err| Error::OpenGitRepo(Box::new(err)))?;
    let references = repo.references()?;
    let tags = references.tags()?.flat_map(|tag| {
        tag.map(|reference| {
            reference
                .name()
                .as_bstr()
                .to_string()
                .replace("refs/tags/", "")
        })
    });
    let mut current_versions = CurrentVersions::default();
    let pattern = prefix
        .as_ref()
        .map_or_else(|| String::from("v"), |prefix| format!("{prefix}/v"));
    for tag in tags {
        if !tag.starts_with(&pattern) {
            continue;
        }
        let version_string = tag.replace(&pattern, "");
        if let Ok(version) = Version::from_str(version_string.as_str()) {
            current_versions.update_version(version);
        }
    }

    Ok(current_versions)
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error("Could not determine current directory: {0}")]
    CurrentDirectory(std::io::Error),
    #[error("Could not open Git repository: {0}")]
    #[diagnostic(
        code(git::open_git_repo),
        help("Please check that the current directory is a Git repository.")
    )]
    OpenGitRepo(#[source] Box<open::Error>),
    #[error("Could not get Git references to parse tags: {0}")]
    GitReferences(#[from] gix::reference::iter::Error),
    #[error("Could not get Git tags: {0}")]
    Tags(#[from] gix::reference::iter::init::Error),
    #[error("Could not find head commit: {0}")]
    HeadCommit(#[from] gix::reference::head_commit::Error),
    #[error("Could not determine Git committer to commit changes")]
    #[diagnostic(
        code(git::no_committer),
        help(
            "We couldn't determine who to commit the changes as. Please set the `user.name` and \
                `user.email` Git config options."
        )
    )]
    NoCommitter,
    #[error("Could not create a tag: {0}")]
    #[diagnostic(
        code(git::tag_failed),
        help("A Git tag could not be created for the release.")
    )]
    CreateTagError(#[from] gix::tag::Error),
    #[error("Failed to write to stdout")]
    Stdout(#[source] std::io::Error),
}
