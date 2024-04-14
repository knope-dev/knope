use miette::Diagnostic;

use super::initialize_state;
use crate::{
    app_config, config,
    integrations::{ureq_err_to_string, ResponseIssue},
    prompt, state,
    step::issues::Issue,
};

pub(crate) fn list_issues(
    config: &Option<config::Gitea>,
    state: state::Gitea,
    labels: Option<&[String]>,
) -> Result<(state::Gitea, Vec<Issue>), Error> {
    let Some(config) = config else {
        return Err(Error::NotConfigured);
    };
    let (token, agent) = initialize_state(&config.host, state)?;
    let labels = labels.unwrap_or(&[]).join(",");

    let issues: Vec<Issue> = agent
        .get(&config.get_issues_url())
        .set("Accept", "aplication/json")
        .query("access_token", &token)
        .query("labels", &labels)
        .query("state", "open")
        .query("limit", "30")
        .call()
        .map_err(|source| Error::ApiRequest {
            err: ureq_err_to_string(source),
            activity: "listing issues".to_string(),
            host: config.host.clone(),
        })?
        .into_json::<Vec<ResponseIssue>>()
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

    Ok((state::Gitea::Initialized { token, agent }, issues))
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
        source: std::io::Error,
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
