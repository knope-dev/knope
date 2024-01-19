use miette::Diagnostic;
use reqwest::{Client, Response};
use serde_json::json;

use super::get_token;
use crate::{
    app_config, config,
    dry_run::DryRun,
    integrations::{git, PullRequest},
    state,
    workflow::Verbose,
};

pub(crate) async fn create_or_update_pull_request(
    title: &str,
    body: &str,
    base: &str,
    state: state::Gitea,
    config: &config::Gitea,
    dry_run: DryRun,
    verbose: Verbose,
    client: Client,
) -> Result<state::Gitea, Error> {
    let branch_ref = git::current_branch()?;
    let current_branch = branch_ref.split('/').last().ok_or(Error::GitRef)?;
    if let Some(stdout) = dry_run {
        writeln!(
            stdout,
            "Would create or update a pull request from {current_branch} to {base}:"
        )
        .map_err(Error::Stdout)?;
        writeln!(stdout, "\tTitle: {title}").map_err(Error::Stdout)?;
        writeln!(stdout, "\tBody: {body}").map_err(Error::Stdout)?;
        return Ok(state);
    }
    let token = get_token(&config.host, state)?;

    let existing_pulls: Vec<PullRequest> = client
        .get(&config.get_pulls_url())
        .header("Accept", "application/json")
        .query(&[
            ("state", "open"),
            (
                "head",
                &format!("{owner}:{current_branch}", owner = config.owner),
            ),
            ("base", base),
            ("access_token", &token),
        ])
        .send()
        .await
        .and_then(Response::error_for_status)
        .map_err(|err| Error::ApiRequest {
            err: err.to_string(),
            activity: "fetching existing pull requests".to_string(),
            host: config.host.clone(),
        })?
        .json()
        .await
        .map_err(|source| Error::ApiResponse {
            source,
            activity: "fetching existing pull requests",
            host: config.host.clone(),
        })?;

    // Update the existing PR
    if let Some(pr) = existing_pulls.first() {
        if let Verbose::Yes = verbose {
            println!("Updating existing pull request: {}", pr.url);
        }
        update_pull_request(client, config, &token, pr.number, title, body).await?;
    // Create a new PR
    } else {
        if let Verbose::Yes = verbose {
            println!("No matching existing pull request found, creating a new one.");
        }
        create_pull_request(
            client,
            config,
            &token,
            verbose,
            base,
            current_branch,
            title,
            body,
        )
        .await?;
    }

    Ok(state::Gitea::Initialized { token })
}

async fn update_pull_request(
    client: Client,
    config: &config::Gitea,
    token: &str,
    number: u32,
    title: &str,
    body: &str,
) -> Result<(), Error> {
    client
        .patch(&config.get_pull_url(number))
        .header("Accept", "application/json")
        .query(&[("access_token", token)])
        .json(&json!({
            "body": body,
            "title": title
        }))
        .send()
        .await
        .and_then(Response::error_for_status)
        .map_err(|source| Error::ApiRequest {
            err: source.to_string(),
            activity: "updating pull request".to_string(),
            host: config.host.clone(),
        })?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn create_pull_request(
    client: Client,
    config: &config::Gitea,
    token: &str,
    verbose: Verbose,
    base: &str,
    head: &str,
    title: &str,
    body: &str,
) -> Result<(), Error> {
    let new_pr = client
        .post(&config.get_pulls_url())
        .header("Accept", "application/json")
        .query(&[("access_token", token)])
        .json(&json!({
            "title": title,
            "body": body,
            "head": head,
            "base": base
        }))
        .send()
        .await
        .and_then(Response::error_for_status)
        .map_err(|source| Error::ApiRequest {
            err: source.to_string(),
            activity: "creating pull request".to_string(),
            host: config.host.clone(),
        })?
        .json::<PullRequest>()
        .await
        .map_err(|source| Error::ApiResponse {
            source,
            activity: "creating pull request",
            host: config.host.clone(),
        })?;

    if let Verbose::Yes = verbose {
        println!("Created new pull request: {pr_url}", pr_url = new_pr.url);
    }
    Ok(())
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("Trouble communicating with the Gitea instance while {activity}: {err}")]
    #[diagnostic(
        code(gitea::api_request_error),
        help(
            "There was a problem communicating with the Gitea instance {host}, this may be a network issue or a permissions issue."
        )
    )]
    ApiRequest {
        err: String,
        activity: String,
        host: String,
    },
    #[error("Trouble decoding the response from Gitea while {activity}: {source}")]
    #[diagnostic(
        code(gitea::api_response_error),
        help(
            "Failure to decode a response from the Gitea instance at {host} is probably a bug. Please report it at https://github.com/knope-dev/knope"
        )
    )]
    ApiResponse {
        source: reqwest::Error,
        activity: &'static str,
        host: String,
    },
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(#[from] git::Error),
    #[error("Trouble getting the head branch")]
    #[diagnostic(
        code(gitea::failed_getting_current_branch),
        help("The current branch could not be parsed from the git ref path. This is a bug, please report it at https://github.com/knope-dev/knope ")
    )]
    GitRef,
    #[error(transparent)]
    #[diagnostic(transparent)]
    AppConfig(#[from] app_config::Error),
    #[error("Error writing to stdout: {0}")]
    Stdout(#[source] std::io::Error),
}
