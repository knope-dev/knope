use miette::Diagnostic;
use serde_json::json;
use tracing::{debug, info};
use ureq::Agent;

use super::initialize_state;
use crate::{
    app_config, config,
    integrations::{ApiRequestError, PullRequest, git, handle_response},
    state,
    state::RunType,
};

pub(crate) fn create_or_update_pull_request(
    title: &str,
    body: &str,
    base: &str,
    state: RunType<state::Gitea>,
    config: &config::Gitea,
) -> Result<state::Gitea, Error> {
    let branch_ref = git::current_branch()?;
    let current_branch = branch_ref.split('/').next_back().ok_or(Error::GitRef)?;
    let state = match state {
        RunType::DryRun(state) => {
            info!("Would create or update a pull request from {current_branch} to {base}:");
            info!("\tTitle: {title}");
            info!("\tBody: {body}");
            return Ok(state);
        }
        RunType::Real(state) => state,
    };
    let (token, agent) = initialize_state(&config.host, state)?;

    let resp = agent
        .get(&config.get_pulls_url())
        .header("Accept", "application/json")
        .query("state", "open")
        .query(
            "head",
            format!("{owner}:{current_branch}", owner = config.owner),
        )
        .query("base", base)
        .query("access_token", &token)
        .call();
    let resp = handle_response(
        resp,
        config.host.clone(),
        "fetching existing pull requests".to_string(),
    )?;
    let existing_pulls: Vec<PullRequest> =
        resp.into_body()
            .read_json()
            .map_err(|source| Error::ApiResponse {
                source,
                activity: "fetching existing pull requests",
                host: config.host.clone(),
            })?;

    // Update the existing PR
    if let Some(pr) = existing_pulls.first() {
        debug!("Updating existing pull request: {}", pr.url);
        update_pull_request(&agent, config, &token, pr.number, title, body)?;
    // Create a new PR
    } else {
        debug!("No matching existing pull request found, creating a new one.");
        create_pull_request(&agent, config, &token, base, current_branch, title, body)?;
    }

    Ok(state::Gitea::Initialized { token, agent })
}

fn update_pull_request(
    agent: &Agent,
    config: &config::Gitea,
    token: &str,
    number: u32,
    title: &str,
    body: &str,
) -> Result<(), Error> {
    let resp = agent
        .patch(&config.get_pull_url(number))
        .header("Accept", "application/json")
        .query("access_token", token)
        .send_json(json!({
            "body": body,
            "title": title
        }));
    handle_response(
        resp,
        config.host.clone(),
        "updating pull request".to_string(),
    )?;
    Ok(())
}

fn create_pull_request(
    agent: &Agent,
    config: &config::Gitea,
    token: &str,
    base: &str,
    head: &str,
    title: &str,
    body: &str,
) -> Result<(), Error> {
    let resp = agent
        .post(&config.get_pulls_url())
        .header("Accept", "application/json")
        .query("access_token", token)
        .send_json(json!({
            "title": title,
            "body": body,
            "head": head,
            "base": base,
        }));
    let resp = handle_response(
        resp,
        config.host.clone(),
        "creating pull request".to_string(),
    )?;
    let new_pr = resp
        .into_body()
        .read_json::<PullRequest>()
        .map_err(|source| Error::ApiResponse {
            source,
            activity: "creating pull request",
            host: config.host.clone(),
        })?;

    debug!("Created new pull request: {pr_url}", pr_url = new_pr.url);
    Ok(())
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    ApiRequest(#[from] ApiRequestError),
    #[error("Trouble decoding the response from Gitea while {activity}: {source}")]
    #[diagnostic(
        code(gitea::api_response_error),
        help(
            "Failure to decode a response from the Gitea instance at {host} is probably a bug. Please report it at https://github.com/knope-dev/knope"
        )
    )]
    ApiResponse {
        source: ureq::Error,
        activity: &'static str,
        host: String,
    },
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(#[from] git::Error),
    #[error("Trouble getting the head branch")]
    #[diagnostic(
        code(gitea::failed_getting_current_branch),
        help(
            "The current branch could not be parsed from the git ref path. This is a bug, please report it at https://github.com/knope-dev/knope "
        )
    )]
    GitRef,
    #[error(transparent)]
    #[diagnostic(transparent)]
    AppConfig(#[from] app_config::Error),
}
