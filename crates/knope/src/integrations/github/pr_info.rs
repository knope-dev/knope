use knope_versioning::changes::Change;
use miette::Diagnostic;
use tracing::{debug, warn};
use ureq::Agent;

use super::initialize_state;
use crate::{
    app_config, config,
    integrations::{ApiRequestError, handle_response},
    state,
};

#[derive(serde::Deserialize)]
struct PullRequestInfo {
    number: u64,
    user: Option<PullRequestUser>,
}

#[derive(serde::Deserialize)]
struct PullRequestUser {
    login: String,
}

/// Enrich each [`Change`]'s git info with PR number and author login from the GitHub API.
///
/// Only changes that already have a commit hash are looked up. The GitHub state is returned
/// so the caller can reuse an initialized token/agent for subsequent API calls.
pub(crate) fn enrich_git_info(
    changes: &mut [Change],
    github_config: &config::GitHub,
    github_state: state::GitHub,
) -> Result<state::GitHub, Error> {
    let indices: Vec<usize> = changes
        .iter()
        .enumerate()
        .filter_map(|(i, change)| change.git.as_ref().map(|_| i))
        .collect();

    if indices.is_empty() {
        return Ok(github_state);
    }

    let (token, agent) = initialize_state(github_state)?;
    let authorization = format!("token {token}");

    for idx in indices {
        let short_hash = match changes.get(idx).and_then(|c| c.git.as_ref()) {
            Some(g) => g.hash.clone(),
            None => continue,
        };
        match fetch_pr_for_commit(&agent, &authorization, github_config, &short_hash) {
            Ok(Some((pr_number, author_login))) => {
                if let Some(info) = changes.get_mut(idx).and_then(|c| c.git.as_mut()) {
                    info.pr_number = Some(pr_number);
                    info.author_login = Some(author_login);
                }
            }
            Ok(None) => {
                debug!("No PR found for commit {short_hash}");
            }
            Err(e) => {
                warn!("Failed to fetch PR info for commit {short_hash}: {e}");
            }
        }
    }

    Ok(state::GitHub::Initialized { token, agent })
}

fn fetch_pr_for_commit(
    agent: &Agent,
    authorization: &str,
    config: &config::GitHub,
    commit_sha: &str,
) -> Result<Option<(u64, String)>, Error> {
    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/commits/{commit_sha}/pulls",
        owner = config.owner,
        repo = config.repo,
    );

    let response = agent
        .get(&url)
        .header("Authorization", authorization)
        .call();

    let response = handle_response(
        response,
        "GitHub".to_string(),
        format!("fetching PRs for commit {commit_sha}"),
    )?;

    let pulls: Vec<PullRequestInfo> =
        response
            .into_body()
            .read_json()
            .map_err(|source| Error::ApiResponse {
                message: source.to_string(),
                activity: format!("reading PR info for commit {commit_sha}"),
            })?;

    Ok(pulls.into_iter().next().map(|pr| {
        let login = pr.user.map_or_else(String::new, |u| u.login);
        (pr.number, login)
    }))
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    AppConfig(#[from] app_config::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    ApiRequest(#[from] ApiRequestError),
    #[error("Trouble decoding the response from GitHub while {activity}: {message}")]
    #[diagnostic(
        code(github::api_response_error),
        help(
            "Failure to decode a response from GitHub is probably a bug. Please report it at https://github.com/knope-dev/knope"
        )
    )]
    ApiResponse { message: String, activity: String },
}
