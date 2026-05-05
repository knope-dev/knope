use knope_versioning::changes::Change;
use miette::Diagnostic;
use tracing::{debug, warn};

use super::initialize_state;
use crate::{app_config, config, integrations::http, state};

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
pub(crate) async fn enrich_git_info(
    changes: &mut [Change],
    github_config: &config::GitHub,
    github_state: state::GitHub,
) -> Result<state::GitHub, Error> {
    let (token, client) = initialize_state(github_state)?;
    let authorization = format!("Bearer {token}");

    for git_info in changes.iter_mut().filter_map(|change| change.git.as_mut()) {
        let short_hash = &git_info.hash;
        match fetch_pr_for_commit(&client, &authorization, github_config, short_hash).await {
            Ok(Some((pr_number, author_login))) => {
                git_info.pr_number = Some(pr_number);
                git_info.author_login = Some(author_login);
            }
            Ok(None) => {
                debug!("No PR found for commit {short_hash}");
            }
            Err(e) => {
                warn!("Failed to fetch PR info for commit {short_hash}: {e}");
            }
        }
    }

    Ok(state::GitHub::Initialized { token, client })
}

async fn fetch_pr_for_commit(
    client: &http::Client,
    authorization: &str,
    config: &config::GitHub,
    commit_sha: &str,
) -> Result<Option<(u64, String)>, Error> {
    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/commits/{commit_sha}/pulls",
        owner = config.owner,
        repo = config.repo,
    );

    let response = client
        .get(&url)
        .header("Authorization", authorization)
        .send()
        .await;

    let response = http::handle_response(
        response,
        "GitHub".to_string(),
        format!("fetching PRs for commit {commit_sha}"),
    )
    .await?;

    let pulls: Vec<PullRequestInfo> =
        response.json().await.map_err(|source| Error::ApiResponse {
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
    ApiRequest(#[from] http::ApiRequestError),
    #[error("Trouble decoding the response from GitHub while {activity}: {message}")]
    #[diagnostic(
        code(github::api_response_error),
        help(
            "Failure to decode a response from GitHub is probably a bug. Please report it at https://github.com/knope-dev/knope"
        )
    )]
    ApiResponse { message: String, activity: String },
}
