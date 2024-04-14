use miette::Diagnostic;
use serde_json::json;
use ureq::Agent;

use crate::{
    app_config, config,
    dry_run::DryRun,
    integrations::{git, github::initialize_state, ureq_err_to_string, PullRequest},
    state,
    workflow::Verbose,
};

pub(crate) fn create_or_update_pull_request(
    title: &str,
    body: &str,
    base: &str,
    state: state::GitHub,
    config: &config::GitHub,
    dry_run: DryRun,
    verbose: Verbose,
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

    let (token, agent) = initialize_state(state)?;
    let config::GitHub { owner, repo } = config;
    let base_url = format!("https://api.github.com/repos/{owner}/{repo}/pulls");
    let authorization_header = format!("Bearer {}", &token);

    let existing_pulls: Vec<PullRequest> = agent
        .get(&base_url)
        .set("Accept", "application/vnd.github+json")
        .set("Authorization", &authorization_header)
        .query("head", &format!("{owner}:{current_branch}"))
        .query("base", base)
        .call()
        .map_err(|err| Error::ApiRequest {
            err: ureq_err_to_string(err),
            activity: "fetching existing pull requests".to_string(),
        })?
        .into_json()
        .map_err(|source| Error::ApiResponse {
            source,
            activity: "fetching existing pull requests",
        })?;
    let agent = if let Some(existing) = existing_pulls.first() {
        if let Verbose::Yes = verbose {
            println!("Updating existing pull request: {}", existing.url);
        }
        update_pull_request(&existing.url, title, body, &authorization_header, agent)
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
            agent,
            verbose,
        )
    }?;
    Ok(state::GitHub::Initialized { token, agent })
}

fn update_pull_request(
    url: &str,
    title: &str,
    body: &str,
    auth_header: &str,
    agent: Agent,
) -> Result<Agent, Error> {
    agent
        .patch(url)
        .set("Accept", "application/vnd.github+json")
        .set("Authorization", auth_header)
        .send_json(json!({
            "title": title,
            "body": body,
        }))
        .map_err(|source| Error::ApiRequest {
            err: ureq_err_to_string(source),
            activity: "updating pull request".to_string(),
        })?;
    Ok(agent)
}

#[allow(clippy::too_many_arguments)]
fn create_pull_request(
    url: &str,
    title: &str,
    body: &str,
    base: &str,
    current_branch: &str,
    auth_header: &str,
    agent: Agent,
    verbose: Verbose,
) -> Result<Agent, Error> {
    let response = agent
        .post(url)
        .set("Accept", "application/vnd.github+json")
        .set("Authorization", auth_header)
        .send_json(json!({
            "title": title,
            "body": body,
            "head": current_branch,
            "base": base,
        }))
        .map_err(|source| Error::ApiRequest {
            err: ureq_err_to_string(source),
            activity: "creating pull request".to_string(),
        })?;
    if let Verbose::Yes = verbose {
        let json_data = response
            .into_json::<serde_json::Value>()
            .map_err(|source| Error::ApiResponse {
                source,
                activity: "creating pull request",
            })?;
        if let Some(new_pr_url) = json_data.get("url") {
            println!("Created new pull request: {new_pr_url}");
        }
    }
    Ok(agent)
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
        source: std::io::Error,
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
