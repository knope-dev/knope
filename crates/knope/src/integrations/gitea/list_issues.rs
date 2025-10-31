use miette::Diagnostic;

use super::initialize_state;
use crate::{
    app_config, config,
    integrations::{ApiRequestError, ResponseIssue, handle_response},
    prompt, state,
    step::issues::Issue,
};

pub(crate) fn list_issues(
    config: Option<&config::Gitea>,
    state: state::Gitea,
    labels: Option<&[String]>,
) -> Result<(state::Gitea, Vec<Issue>), Error> {
    let Some(config) = config else {
        return Err(Error::NotConfigured);
    };
    let (token, agent) = initialize_state(&config.host, state)?;
    let labels = labels.unwrap_or(&[]).join(",");

    let resp = agent
        .get(&config.get_issues_url())
        .header("Accept", "aplication/json")
        .query("access_token", &token)
        .query("labels", &labels)
        .query("state", "open")
        .query("limit", "30")
        .call();
    let resp = handle_response(resp, config.host.clone(), "listing issues".into())?;

    let issues: Vec<Issue> = resp
        .into_body()
        .read_json::<Vec<ResponseIssue>>()
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
    Prompt(#[from] prompt::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    AppConfig(#[from] app_config::Error),
}
