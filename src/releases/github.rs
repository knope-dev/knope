use std::io::Write;

use serde::Serialize;

use crate::{
    app_config::get_or_prompt_for_github_token,
    config::GitHub,
    releases::{git::tag_name, Release},
    state,
    state::GitHub::{Initialized, New},
    step::StepError,
};

pub(crate) fn release(
    package_name: &Option<String>,
    release: &Release,
    github_state: state::GitHub,
    github_config: &GitHub,
    dry_run_stdout: Option<&mut Box<dyn Write>>,
) -> Result<state::GitHub, StepError> {
    let version = &release.new_version;
    let release_title = release.title()?;

    let tag_name = tag_name(version, package_name);
    let name = if let Some(package_name) = package_name {
        format!("{package_name} {release_title}")
    } else {
        release_title
    };

    let body = release
        .new_changelog
        .lines()
        .map(|line| {
            if line.starts_with("##") {
                #[allow(clippy::indexing_slicing)] // Just checked len above
                &line[1..] // Reduce header level by one
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let github_release = GitHubRelease {
        tag_name: &tag_name,
        name: &name,
        body,
        prerelease: version.is_prerelease(),
    };

    if let Some(stdout) = dry_run_stdout {
        let release_type = if github_release.prerelease {
            "prerelease"
        } else {
            "release"
        };
        writeln!(
            stdout,
            "Would create a {} on GitHub with name {} and tag {} and body:\n{}",
            release_type, name, github_release.tag_name, github_release.body
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
        .send_json(github_release)
        .or(Err(StepError::ApiRequestError))?;

    if response.status() != 201 {
        return Err(StepError::ApiResponseError(None));
    }
    Ok(Initialized { token })
}

#[derive(Serialize)]
struct GitHubRelease<'a> {
    tag_name: &'a str,
    name: &'a str,
    body: String,
    prerelease: bool,
}
