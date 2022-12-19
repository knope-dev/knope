use std::env::current_dir;
use std::io::Write;
use std::str::FromStr;

use git_repository::object::Kind;
use git_repository::open;
use git_repository::refs::transaction::PreviousValue;

use crate::releases::semver::Version;
use crate::releases::{CurrentVersions, Release};
use crate::step::StepError;

pub(crate) fn tag_name(version: &Version, package_name: &Option<String>) -> String {
    let prefix = package_name
        .as_ref()
        .map_or_else(|| "v".to_string(), |name| format!("{name}/v"));
    format!("{prefix}{version}")
}

pub(crate) fn release(
    dry_run_stdout: Option<&mut Box<dyn Write>>,
    release: &Release,
) -> Result<(), StepError> {
    let Release {
        version,
        changelog: _changelog,
        package_name,
    } = release;
    let tag = tag_name(version, package_name);

    if let Some(stdout) = dry_run_stdout {
        writeln!(stdout, "Would create Git tag {tag}")?;
        return Ok(());
    }

    let repo = open(current_dir()?).map_err(|_e| StepError::NotAGitRepo)?;
    let head = repo.head_commit()?;
    repo.tag(
        tag,
        head.id,
        Kind::Commit,
        Some(repo.committer_or_default()),
        "",
        PreviousValue::Any,
    )?;

    Ok(())
}

pub(crate) fn get_current_versions_from_tag(
    prefix: Option<&str>,
) -> Result<CurrentVersions, StepError> {
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
            if version.pre.is_none() {
                current_versions.replace_stable_if_newer(version);
            } else {
                current_versions.insert_prerelease(version);
            }
        }
    }

    Ok(current_versions)
}
