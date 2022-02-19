use color_eyre::eyre::{eyre, WrapErr};
use color_eyre::Result;
use serde_json::json;

use crate::app_config::get_or_prompt_for_github_token;
use crate::state::{GitHub, ReleasePrepared, State};

pub(crate) fn release(state: State) -> Result<State> {
    let release_prepared = match state {
        State::ReleasePrepared(release_prepared) => release_prepared,
        _ => return Err(eyre!("PrepareRelease needs to be called before Release")),
    };
    let github_config = if let Some(github_config) = release_prepared.github_config {
        github_config
    } else {
        return Err(eyre!("PrepareRelease needs to be called before Release"));
    };
    let token = match release_prepared.github_state {
        GitHub::Initialized { token } => token,
        GitHub::New => get_or_prompt_for_github_token()?,
    };

    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/releases",
        owner = github_config.owner,
        repo = github_config.repo,
    );
    let token_header = format!("token {}", &token);

    let response = ureq::post(&url)
        .set("Authorization", &token_header)
        .send_json(json!({
            "tag_name": &release_prepared.new_version,
            "name": &release_prepared.new_version,
            "body": &release_prepared.release_notes,
            "prerelease": release_prepared.is_prerelease,
        }))
        .wrap_err("Could not send release request to GitHub")?;

    if response.status() != 201 {
        return Err(eyre!(
            "Could not create release on GitHub: {}",
            response.into_string().unwrap()
        ));
    }

    Ok(State::ReleasePrepared(ReleasePrepared {
        jira_config: release_prepared.jira_config,
        github_state: GitHub::Initialized { token },
        github_config: Some(github_config),
        release_notes: release_prepared.release_notes,
        new_version: release_prepared.new_version,
        is_prerelease: release_prepared.is_prerelease,
    }))
}
