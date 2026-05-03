use miette::Diagnostic;
use serde::Serialize;
use tracing::info;

use super::initialize_state;
use crate::{
    app_config, config,
    integrations::{ApiRequestError, git, http::handle_response},
    state,
    state::RunType,
};

/// Release creation payload for the Gitea/Forgejo API.
///
/// Intentionally omits `generate_release_notes` (a GitHub-specific field) since sending
/// it to Forgejo can cause the API to fail when trying to process that field.
#[derive(Serialize)]
struct GiteaReleaseInput<'a> {
    tag_name: &'a str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a str>,
    prerelease: bool,
    draft: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_commitish: Option<&'a str>,
}

pub(crate) async fn create_release(
    name: &str,
    tag_name: &str,
    body: &str,
    prerelease: bool,
    gitea_state: RunType<state::Gitea>,
    gitea_config: &config::Gitea,
) -> Result<state::Gitea, Error> {
    let target_commitish = git::get_head_commit_sha().ok();
    let body = if body.is_empty() { None } else { Some(body) };
    let gitea_release = GiteaReleaseInput {
        tag_name,
        name,
        body,
        prerelease,
        draft: false,
        target_commitish: target_commitish.as_deref(),
    };

    let gitea_state = match gitea_state {
        RunType::DryRun(state) => {
            gitea_release_dry_run(name, gitea_config, &gitea_release);
            return Ok(state);
        }
        RunType::Real(gitea_state) => gitea_state,
    };

    let (token, client) = initialize_state(&gitea_config.host, gitea_state)?;

    let resp = client
        .post(gitea_config.get_releases_url())
        .header("Authorization", format!("Bearer {token}"))
        .json(&gitea_release)
        .send()
        .await;
    handle_response(
        resp,
        gitea_config.host.clone(),
        "creating a release".to_string(),
    )
    .await?;

    Ok(state::Gitea::Initialized { token, client })
}

fn gitea_release_dry_run(name: &str, config: &config::Gitea, gitea_release: &GiteaReleaseInput) {
    let release_type = if gitea_release.prerelease {
        "prerelease"
    } else {
        "release"
    };
    let body = gitea_release.body.as_ref().map_or_else(
        || String::from("(no release notes)"),
        |body| format!("body:\n{body}"),
    );
    let target = gitea_release
        .target_commitish
        .map_or_else(String::new, |sha| format!(" at commit {sha}"));
    info!(
        "Would create a {release_type} on Gitea [{host}] with name {name} and tag {tag}{target} and {body}",
        tag = gitea_release.tag_name,
        host = config.host
    );
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    AppConfig(#[from] app_config::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    ApiRequest(#[from] ApiRequestError),
}
