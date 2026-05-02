use miette::Diagnostic;
use serde_json::json;
use tracing::{debug, info};

use crate::{
    app_config, config,
    integrations::{
        ApiRequestError, PullRequest, git,
        github::initialize_state,
        http::{Client, handle_response},
    },
    state::{self, RunType},
};

pub(crate) async fn create_or_update_pull_request(
    title: &str,
    body: &str,
    base: &str,
    state: RunType<state::GitHub>,
    config: &config::GitHub,
) -> Result<state::GitHub, Error> {
    let current_branch = git::current_branch()?;
    let state = match state {
        RunType::DryRun(state) => {
            info!("Would create or update a pull request from {current_branch} to {base}:");
            info!("\tTitle: {title}");
            info!("\tBody: {body}");
            return Ok(state);
        }
        RunType::Real(state) => state,
    };

    let (token, client) = initialize_state(state)?;
    let config::GitHub { owner, repo } = config;
    let base_url = format!("https://api.github.com/repos/{owner}/{repo}/pulls");
    let authorization_header = format!("Bearer {}", &token);

    let existing_pulls_response = client
        .get(&base_url)
        .header("Authorization", &authorization_header)
        .header("Accept", "application/vnd.github+json")
        .query(&[
            ("head", format!("{owner}:{current_branch}").as_str()),
            ("base", base),
        ])
        .send()
        .await;
    let existing_pulls_response = handle_response(
        existing_pulls_response,
        "GitHub".to_string(),
        "fetching existing pull requests".into(),
    )
    .await?;

    let existing_pulls: Vec<PullRequest> =
        existing_pulls_response
            .json()
            .await
            .map_err(|source| Error::ApiResponse {
                source,
                activity: "fetching existing pull requests",
            })?;
    let client = if let Some(existing) = existing_pulls.first() {
        debug!("Updating existing pull request: {}", existing.url);
        update_pull_request(&existing.url, title, body, &authorization_header, client).await
    } else {
        debug!("No matching existing pull request found, creating a new one.");
        create_pull_request(
            &base_url,
            title,
            body,
            base,
            &current_branch,
            &authorization_header,
            client,
        )
        .await
    }?;
    Ok(state::GitHub::Initialized { token, client })
}

async fn update_pull_request(
    url: &str,
    title: &str,
    body: &str,
    auth_header: &str,
    client: Client,
) -> Result<Client, Error> {
    let resp = client
        .patch(url)
        .header("Authorization", auth_header)
        .header("Accept", "application/vnd.github+json")
        .json(&json!({
            "title": title,
            "body": body,
        }))
        .send()
        .await;
    handle_response(resp, "GitHub".to_string(), "updating pull request".into()).await?;
    Ok(client)
}

async fn create_pull_request(
    url: &str,
    title: &str,
    body: &str,
    base: &str,
    current_branch: &str,
    auth_header: &str,
    client: Client,
) -> Result<Client, Error> {
    let response = client
        .post(url)
        .header("Authorization", auth_header)
        .header("Accept", "application/vnd.github+json")
        .json(&json!({
            "title": title,
            "body": body,
            "head": current_branch,
            "base": base,
        }))
        .send()
        .await;
    let response = handle_response(
        response,
        "GitHub".to_string(),
        "creating pull request".to_string(),
    )
    .await?;
    let json_data = response
        .json::<serde_json::Value>()
        .await
        .map_err(|source| Error::ApiResponse {
            source,
            activity: "creating pull request",
        })?;
    if let Some(new_pr_url) = json_data.get("url") {
        debug!("Created new pull request: {new_pr_url}");
    }
    Ok(client)
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    ApiRequest(#[from] ApiRequestError),
    #[error("Trouble decoding the response from GitHub while {activity}: {source}")]
    #[diagnostic(
        code(github::api_response_error),
        help(
            "Failure to decode a response from GitHub is probably a bug. Please report it at https://github.com/knope-dev/knope"
        )
    )]
    ApiResponse {
        source: reqwest::Error,
        activity: &'static str,
    },
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(#[from] git::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    AppConfig(#[from] app_config::Error),
}
