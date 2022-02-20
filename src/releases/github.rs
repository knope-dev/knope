use color_eyre::eyre::{eyre, WrapErr};
use color_eyre::Result;
use serde_json::json;

use crate::app_config::get_or_prompt_for_github_token;
use crate::state::{GitHub, Release, State};

pub(crate) fn release(state: State) -> Result<State> {
    let release = match state.release {
        Release::Prepared(release) => release,
        _ => return Err(eyre!("PrepareRelease needs to be called before Release.")),
    };
    let github_config = if let Some(github_config) = state.github_config {
        github_config
    } else {
        return Err(eyre!("GitHub needs to be configured."));
    };
    let token = match state.github {
        GitHub::Initialized { token } => token,
        GitHub::New => get_or_prompt_for_github_token()?,
    };

    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/releases",
        owner = github_config.owner,
        repo = github_config.repo,
    );
    let token_header = format!("token {}", &token);

    let version_string = release.version.to_string();
    let response = ureq::post(&url)
        .set("Authorization", &token_header)
        .send_json(json!({
            "tag_name": &version_string,
            "name": &version_string,
            "body": &release.changelog,
            "prerelease": !release.version.pre.is_empty(),
        }))
        .wrap_err("Could not send release request to GitHub")?;

    if response.status() != 201 {
        return Err(eyre!(
            "Could not create release on GitHub: {}",
            response.into_string().unwrap()
        ));
    }

    Ok(State {
        jira_config: state.jira_config,
        github: GitHub::Initialized { token },
        github_config: Some(github_config),
        issue: state.issue,
        release: Release::Prepared(release),
    })
}
