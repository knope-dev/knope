use std::env::current_dir;
use std::io::Write;

use git_repository::object::Kind;
use git_repository::open;
use git_repository::refs::transaction::PreviousValue;
use semver::Version;

use crate::releases::{CurrentVersions, Release};
use crate::step::StepError;
use crate::{RunType, State};

pub(crate) fn release(
    state: State,
    dry_run_stdout: Option<Box<dyn Write>>,
    release: &Release,
) -> Result<RunType, StepError> {
    let version_string = release.version.to_string();
    let tag = format!("v{}", version_string);

    if let Some(mut stdout) = dry_run_stdout {
        writeln!(stdout, "Would create Git tag {}", tag)?;
        return Ok(RunType::DryRun { stdout, state });
    }

    let repo = open(current_dir()?).map_err(|_e| StepError::NotAGitRepo)?;
    let head = repo.head_commit()?;
    repo.tag(tag, head.id, Kind::Commit, None, "", PreviousValue::Any)?;

    Ok(RunType::Real(state))
}

pub(crate) fn get_current_versions_from_tag() -> Result<Option<CurrentVersions>, StepError> {
    let repo = open(current_dir()?).map_err(|_e| StepError::NotAGitRepo)?;
    let references = repo.references().map_err(|_e| StepError::NotAGitRepo)?;
    let tags = references
        .tags()
        .map_err(|_e| StepError::NotAGitRepo)?
        .flat_map(|tag| {
            tag.map(|reference| {
                reference
                    .name()
                    .as_bstr()
                    .to_string()
                    .split('/')
                    .last()
                    .map(String::from)
            })
        })
        .flatten();
    let mut stable = None;
    let mut prerelease = None;
    for tag in tags {
        if !tag.starts_with('v') {
            continue;
        }
        if let Ok(version) = Version::parse(&tag[1..tag.len()]) {
            if version.pre.is_empty() {
                stable = Some(version);
                prerelease = None; // Don't consider prereleases older than the stable version.
            } else {
                prerelease.get_or_insert(version);
            }
        }
    }
    Ok(stable.map(|stable| CurrentVersions { stable, prerelease }))
}
