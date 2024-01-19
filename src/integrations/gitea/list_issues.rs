use miette::Diagnostic;
use reqwest::{Client, Response};

use super::get_token;
use crate::{app_config, config, integrations::ResponseIssue, prompt, state, step::issues::Issue};

pub(crate) async fn list_issues(
    config: &Option<config::Gitea>,
    state: state::Gitea,
    labels: Option<&[String]>,
    client: Client,
) -> Result<(state::Gitea, Vec<Issue>), Error> {
    let Some(config) = config else {
        return Err(Error::NotConfigured);
    };
    let token = get_token(&config.host, state)?;
    let labels = labels.unwrap_or(&[]).join(",");

    let issues: Vec<Issue> = client
        .get(&config.get_issues_url())
        .header("Accept", "aplication/json")
        .query(&[
            ("state", "open"),
            ("access_token", &token),
            ("labels", &labels),
            ("limit", "30"),
        ])
        .send()
        .await
        .and_then(Response::error_for_status)
        .map_err(|source| Error::ApiRequest {
            err: source.to_string(),
            activity: "listing issues".to_string(),
            host: config.host.clone(),
        })?
        .json::<Vec<ResponseIssue>>()
        .await
        .map_err(|source| Error::ApiResponse {
            source,
            activity: "listing issues",
            host: config.host.clone(),
        })?
        .into_iter()
        .map(|response| Issue {
            key: response.number.to_string(),
            summary: response.title,
        })
        .collect();

    Ok((state::Gitea::Initialized { token }, issues))
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("Gitea is not configured")]
    #[diagnostic(
        code(issues::gitea::not_configured),
        help("Gitea must be configured in order to use the SelectGiteaIssue step"),
        url("https://knope.tech/reference/config-file/gitea/")
    )]
    NotConfigured,
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
    #[error("Could not write to stdout")]
    Stdout(#[from] std::io::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Prompt(#[from] prompt::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    AppConfig(#[from] app_config::Error),
}
