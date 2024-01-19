use miette::Diagnostic;
use reqwest::{Client, Response};
use serde_json::json;

use crate::{
    app_config, config,
    dry_run::DryRun,
    integrations::{git, github::initialize_state, PullRequest},
    state,
    workflow::Verbose,
};

pub(crate) async fn create_or_update_pull_request(
    title: &str,
    body: &str,
    base: &str,
    state: state::GitHub,
    config: &config::GitHub,
    dry_run: DryRun,
    verbose: Verbose,
    client: Client,
) -> Result<state::GitHub, Error> {
    let current_branch = git::current_branch()?;
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

    let token = initialize_state(state)?;
    let config::GitHub { owner, repo } = config;
    let base_url = format!("https://api.github.com/repos/{owner}/{repo}/pulls");
    let authorization_header = format!("Bearer {}", &token);

    let existing_pulls: Vec<PullRequest> = client
        .get(&base_url)
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", &authorization_header)
        .query(&[
            ("head", &format!("{owner}:{current_branch}")),
            ("base", &base),
        ])
        .send()
        .await
        .and_then(Response::error_for_status)
        .map_err(|err| Error::ApiRequest {
            err: err.to_string(),
            activity: "fetching existing pull requests".to_string(),
        })?
        .json()
        .await
        .map_err(|source| Error::ApiResponse {
            source,
            activity: "fetching existing pull requests",
        })?;
    if let Some(existing) = existing_pulls.first() {
        if let Verbose::Yes = verbose {
            println!("Updating existing pull request: {}", existing.url);
        }
        update_pull_request(&existing.url, title, body, &authorization_header, client).await
    } else {
        if let Verbose::Yes = verbose {
            println!("No matching existing pull request found, creating a new one.");
        }
        create_pull_request(
            &base_url,
            title,
            body,
            base,
            &current_branch,
            &authorization_header,
            client,
            verbose,
        )
        .await
    }?;
    Ok(state::GitHub::Initialized { token })
}

async fn update_pull_request(
    url: &str,
    title: &str,
    body: &str,
    auth_header: &str,
    client: Client,
) -> Result<(), Error> {
    client
        .patch(url)
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", auth_header)
        .json(&json!({
            "title": title,
            "body": body,
        }))
        .send()
        .await
        .and_then(Response::error_for_status)
        .map_err(|source| Error::ApiRequest {
            err: source.to_string(),
            activity: "updating pull request".to_string(),
        })?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn create_pull_request(
    url: &str,
    title: &str,
    body: &str,
    base: &str,
    current_branch: &str,
    auth_header: &str,
    client: Client,
    verbose: Verbose,
) -> Result<(), Error> {
    let response = client
        .post(url)
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", auth_header)
        .json(&json!({
            "title": title,
            "body": body,
            "head": current_branch,
            "base": base,
        }))
        .send()
        .await
        .and_then(Response::error_for_status)
        .map_err(|source| Error::ApiRequest {
            err: source.to_string(),
            activity: "creating pull request".to_string(),
        })?;
    if let Verbose::Yes = verbose {
        let json_data = response
            .json::<serde_json::Value>()
            .await
            .map_err(|source| Error::ApiResponse {
                source,
                activity: "creating pull request",
            })?;
        if let Some(new_pr_url) = json_data.get("url") {
            println!("Created new pull request: {new_pr_url}");
        }
    }
    Ok(())
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("Trouble communicating with GitHub while {activity}: {err}")]
    #[diagnostic(
        code(github::api_request_error),
        help(
            "There was a problem communicating with GitHub, this may be a network issue or a permissions issue."
        )
    )]
    ApiRequest { err: String, activity: String },
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
    #[error("Error writing to stdout: {0}")]
    Stdout(#[source] std::io::Error),
}
