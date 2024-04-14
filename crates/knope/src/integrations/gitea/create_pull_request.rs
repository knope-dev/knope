use miette::Diagnostic;
use serde_json::json;
use ureq::Agent;

use super::initialize_state;
use crate::{
    app_config, config,
    dry_run::DryRun,
    integrations::{git, ureq_err_to_string, PullRequest},
    state,
    workflow::Verbose,
};

pub(crate) fn create_or_update_pull_request(
    title: &str,
    body: &str,
    base: &str,
    state: state::Gitea,
    config: &config::Gitea,
    dry_run: DryRun,
    verbose: Verbose,
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
    let (token, agent) = initialize_state(&config.host, state)?;

    let existing_pulls: Vec<PullRequest> = agent
        .get(&config.get_pulls_url())
        .set("Accept", "application/json")
        .query("state", "open")
        .query(
            "head",
            &format!("{owner}:{current_branch}", owner = config.owner),
        )
        .query("base", base)
        .query("access_token", &token)
        .call()
        .map_err(|err| Error::ApiRequest {
            err: ureq_err_to_string(err),
            activity: "fetching existing pull requests".to_string(),
            host: config.host.clone(),
        })?
        .into_json()
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
        update_pull_request(&agent, config, &token, pr.number, title, body)?;
    // Create a new PR
    } else {
        if let Verbose::Yes = verbose {
            println!("No matching existing pull request found, creating a new one.");
        }
        create_pull_request(
            &agent,
            config,
            &token,
            verbose,
            base,
            current_branch,
            title,
            body,
        )?;
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
    agent
        .patch(&config.get_pull_url(number))
        .set("Accept", "application/json")
        .query("access_token", token)
        .send_json(json!({
            "body": body,
            "title": title
        }))
        .map_err(|source| Error::ApiRequest {
            err: ureq_err_to_string(source),
            activity: "updating pull request".to_string(),
            host: config.host.clone(),
        })?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn create_pull_request(
    agent: &Agent,
    config: &config::Gitea,
    token: &str,
    verbose: Verbose,
    base: &str,
    head: &str,
    title: &str,
    body: &str,
) -> Result<(), Error> {
    let new_pr = agent
        .post(&config.get_pulls_url())
        .set("Accept", "application/json")
        .query("access_token", token)
        .send_json(json!({
            "title": title,
            "body": body,
            "head": head,
            "base": base
        }))
        .map_err(|source| Error::ApiRequest {
            err: ureq_err_to_string(source),
            activity: "creating pull request".to_string(),
            host: config.host.clone(),
        })?
        .into_json::<PullRequest>()
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
        source: std::io::Error,
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
