use std::io::Write;

use serde::Serialize;

use crate::app_config::get_or_prompt_for_github_token;
use crate::config::GitHub;
use crate::releases::git::tag_name;
use crate::releases::Release;
use crate::state;
use crate::state::GitHub::{Initialized, New};
use crate::step::StepError;

pub(crate) fn release(
    release: &Release,
    github_state: state::GitHub,
    github_config: &GitHub,
    dry_run_stdout: Option<&mut Box<dyn Write>>,
) -> Result<state::GitHub, StepError> {
    let Release {
        version,
        changelog,
        package_name,
    } = release;
    let version_string = release.version.to_string();

    let tag_name = tag_name(version, package_name);
    let name = if let Some(package_name) = package_name {
        format!("{package_name} {version_string}")
    } else {
        version_string
    };

    let github_release = GitHubRelease {
        tag_name: &tag_name,
        name: &name,
        body: changelog,
        prerelease: release.version.is_prerelease(),
    };

    if let Some(stdout) = dry_run_stdout {
        let release_type = if github_release.prerelease {
            "prerelease"
        } else {
            "release"
        };
        writeln!(
            stdout,
            "Would create a {} on GitHub with name and tag {} and body:\n{}",
            release_type, github_release.tag_name, github_release.body
        )?;
        return Ok(github_state);
    }

    let token = match github_state {
        Initialized { token } => token,
        New => get_or_prompt_for_github_token()?,
    };

    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/releases",
        owner = github_config.owner,
        repo = github_config.repo,
    );
    let token_header = format!("token {}", &token);

    let response = ureq::post(&url)
        .set("Authorization", &token_header)
        .send_json(github_release)?;

    if response.status() != 201 {
        return Err(StepError::ApiResponseError(None));
    }
    Ok(Initialized { token })
}

#[derive(Serialize)]
struct GitHubRelease<'a> {
    tag_name: &'a str,
    name: &'a str,
    body: &'a str,
    prerelease: bool,
}
