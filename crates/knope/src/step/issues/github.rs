use miette::Diagnostic;
use tracing::info;
use ureq::Agent;

use super::Issue;
use crate::{
    app_config,
    app_config::get_or_prompt_for_github_token,
    config, prompt,
    prompt::select,
    state,
    state::{RunType, State},
};

const ISSUES_QUERY: &str = r"
query($repo: String!, $owner: String!, $labels: [String!]) {
  repository(name:$repo, owner:$owner) {
    issues(states:OPEN, first: 30, labels: $labels) {
      nodes {
        number,
        title
      }
    }
  }
}
";

#[derive(serde::Deserialize)]
struct ResponseIssue {
    number: usize,
    title: String,
}

pub(crate) fn select_issue(
    labels: Option<&[String]>,
    state: RunType<State>,
) -> Result<RunType<State>, Error> {
    match state {
        RunType::DryRun(mut state) => {
            if state.github_config.is_none() {
                return Err(Error::NotConfigured);
            }
            if let Some(labels) = labels {
                info!(
                    "Would query configured GitHub instance for issues with labels {}",
                    labels.join(", ")
                );
            } else {
                info!("Would query configured GitHub instance for issues with any labels");
            }
            info!("Would prompt user to select an issue and move workflow to IssueSelected state.");
            state.issue = state::Issue::Selected(Issue {
                key: String::from("123"),
                summary: String::from("Test issue"),
            });
            Ok(RunType::DryRun(state))
        }
        RunType::Real(state) => {
            let github_config = state.github_config.as_ref().ok_or(Error::NotConfigured)?;
            let (github, issues) = list_issues(github_config, state.github, labels)?;
            let issue = select(issues, "Select an Issue")?;
            info!("Selected item : {}", &issue);
            Ok(RunType::Real(State {
                github,
                issue: state::Issue::Selected(issue),
                ..state
            }))
        }
    }
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("GitHub is not configured")]
    #[diagnostic(
        code(issues::github::not_configured),
        help("GitHub must be configured in order to use the SelectGitHubIssue step"),
        url("https://knope.tech/reference/config-file/github/")
    )]
    NotConfigured,
    #[error("Could not communicate with GitHub while {context}: {source}")]
    #[diagnostic(
        code(issues::github::api),
        help("Check your network connection and GitHub configuration"),
        url("https://knope.tech/reference/config-file/github/")
    )]
    Api {
        source: Box<ureq::Error>,
        context: &'static str,
    },
    #[error("I/O error encountered when communicating with GitHub: {0}")]
    #[diagnostic(code(issues::github::api_io), help("Check your network connection"))]
    ApiIo(std::io::Error),
    #[error("Could not deserialize response from GitHub into JSON: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Received unexpected data from GitHub: {0}")]
    #[diagnostic(
        code(issues::github::unexpected_response),
        help("It's possible GitHub has updated their API, please report this issue")
    )]
    UnexpectedApiResponse(String),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Prompt(#[from] prompt::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    AppConfig(#[from] app_config::Error),
}

fn list_issues(
    github_config: &config::GitHub,
    github_state: state::GitHub,
    labels: Option<&[String]>,
) -> Result<(state::GitHub, Vec<Issue>), Error> {
    let (token, agent) = match github_state {
        state::GitHub::Initialized { token, agent } => (token, agent),
        state::GitHub::New => (
            get_or_prompt_for_github_token()?,
            Agent::new_with_defaults(),
        ),
    };
    let json_value: serde_json::Value = agent
        .post("https://api.github.com/graphql")
        .header("Authorization", &format!("bearer {token}"))
        .send_json(serde_json::json!({
            "query": ISSUES_QUERY,
            "variables": {
                "repo": github_config.repo,
                "owner": github_config.owner,
                "labels": labels
            }
        }))
        .map_err(|source| Error::Api {
            source: Box::new(source),
            context: "loading issues",
        })?
        .body_mut()
        .read_json()
        .map_err(|e| {
            Error::ApiIo(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?;

    let gh_issues = decode_github_response(&json_value)?;

    let issues = gh_issues
        .into_iter()
        .map(|gh_issue| Issue {
            key: gh_issue.number.to_string(),
            summary: gh_issue.title,
        })
        .collect();

    Ok((state::GitHub::Initialized { token, agent }, issues))
}

fn decode_github_response(json_value: &serde_json::Value) -> Result<Vec<ResponseIssue>, Error> {
    let json_issues = json_value.pointer("/data/repository/issues/nodes");
    match json_issues {
        Some(value) => serde_json::from_value(value.clone()).map_err(Error::from),
        None => Err(Error::UnexpectedApiResponse(json_value.to_string())),
    }
}
