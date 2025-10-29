use base64::{Engine, prelude::BASE64_STANDARD as base64};
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::Issue;
use crate::{
    app_config,
    app_config::{get_or_prompt_for_email, get_or_prompt_for_jira_token},
    config::Jira,
    prompt,
    prompt::select,
    state,
    state::{RunType, State},
};

pub(crate) fn select_issue(status: &str, state: RunType<State>) -> Result<RunType<State>, Error> {
    let (run_type, mut state) = state.take();
    let jira_config = state.jira_config.as_ref().ok_or(Error::NotConfigured)?;

    if let RunType::DryRun(()) = run_type {
        info!("Would query configured Jira instance for issues with status {status}");
        info!("Would prompt user to select an issue and move workflow to IssueSelected state.");
        state.issue = state::Issue::Selected(Issue {
            key: "FAKE-123".to_string(),
            summary: "Test issue".to_string(),
        });
        return Ok(RunType::DryRun(state));
    }

    let issues = get_issues(jira_config, status)?;
    let issue = select(issues, "Select an Issue")?;
    info!("Selected item : {}", &issue);
    state.issue = state::Issue::Selected(issue);
    Ok(RunType::Real(state))
}

pub(crate) fn transition_issue(
    status: &str,
    state: RunType<State>,
) -> Result<RunType<State>, Error> {
    let (run_type, state) = state.take();
    let issue = match &state.issue {
        state::Issue::Selected(issue) => issue,
        state::Issue::Initial => return Err(Error::NoIssueSelected),
    };
    let jira_config = state.jira_config.as_ref().ok_or(Error::NotConfigured)?;

    if let RunType::DryRun(()) = run_type {
        info!("Would transition currently selected issue to status {status}");
        return Ok(RunType::DryRun(state));
    }

    run_transition(jira_config, &issue.key, status)?;
    let key = &issue.key;
    info!("{key} transitioned to {status}");
    Ok(RunType::Real(state))
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("Jira is not configured")]
    #[diagnostic(
        code(issues::jira::not_configured),
        help("Jira must be configured in order to select a Jira issue"),
        url("https://knope.tech/reference/config-file/jira/")
    )]
    NotConfigured,
    #[error("Error communicating with API")]
    Io(#[from] std::io::Error),
    #[error("Problem communicating with Jira while {activity}: {inner}")]
    Api {
        activity: &'static str,
        #[source]
        inner: Box<ureq::Error>,
    },
    #[error("The specified transition name was not found in the Jira project")]
    #[diagnostic(
        code(issues::jira::transition),
        help(
            "The `transition` field in TransitionJiraIssue must correspond to a valid transition in the Jira project"
        ),
        url("https://knope.tech/reference/config-file/jira/")
    )]
    Transition,
    #[error("No issue selected")]
    #[diagnostic(
        code(issues::jira::no_issue_selected),
        help(
            "You must use the SelectJiraIssue step before TransitionJiraIssue in the same workflow"
        )
    )]
    NoIssueSelected,
    #[error(transparent)]
    #[diagnostic(transparent)]
    AppConfig(#[from] app_config::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Prompt(#[from] prompt::Error),
}

#[derive(Deserialize, Debug)]
struct IssueFields {
    summary: String,
}

#[derive(Deserialize, Debug)]
struct JiraIssue {
    key: String,
    fields: IssueFields,
}

#[derive(Deserialize, Debug)]
struct SearchResponse {
    issues: Vec<JiraIssue>,
}

fn get_auth() -> Result<String, Error> {
    let email = get_or_prompt_for_email()?;
    let token = get_or_prompt_for_jira_token()?;
    Ok(format!(
        "Basic {}",
        base64.encode(format!("{email}:{token}"))
    ))
}

pub(crate) fn get_issues(jira_config: &Jira, status: &str) -> Result<Vec<Issue>, Error> {
    let auth = get_auth()?;
    let project = &jira_config.project;
    let jql = format!("status = {status} AND project = {project}");
    let url = format!("{}/rest/api/3/search", jira_config.url);
    let mut response = ureq::post(&url)
        .header("Authorization", &auth)
        .send_json(serde_json::json!({"jql": jql, "fields": ["summary"]}))
        .map_err(|inner| Error::Api {
            inner: Box::new(inner),
            activity: "querying for issues",
        })?;
    let search_response: SearchResponse =
        response
            .body_mut()
            .read_json()
            .map_err(|inner| Error::Api {
                inner: Box::new(inner),
                activity: "parsing search response",
            })?;
    Ok(search_response
        .issues
        .into_iter()
        .map(|jira_issue| Issue {
            key: jira_issue.key,
            summary: jira_issue.fields.summary,
        })
        .collect())
}

fn run_transition(jira_config: &Jira, issue_key: &str, status: &str) -> Result<(), Error> {
    let auth = get_auth()?; // TODO: get auth once and store in state
    let base_url = &jira_config.url;
    let url = format!("{base_url}/rest/api/3/issue/{issue_key}/transitions",);
    let agent = ureq::Agent::new_with_defaults();
    let mut response = agent
        .get(&url)
        .header("Authorization", &auth)
        .call()
        .map_err(|inner| Error::Api {
            inner: Box::new(inner),
            activity: "getting transitions",
        })?;
    let response: GetTransitionResponse =
        response
            .body_mut()
            .read_json()
            .map_err(|inner| Error::Api {
                inner: Box::new(inner),
                activity: "parsing transitions response",
            })?;
    let transition = response
        .transitions
        .into_iter()
        .find(|transition| transition.name == status)
        .ok_or(Error::Transition)?;
    let _response = agent
        .post(&url)
        .header("Authorization", &auth)
        .send_json(serde_json::json!({"transition": {"id": transition.id}}))
        .map_err(|inner| Error::Api {
            inner: Box::new(inner),
            activity: "transitioning issue",
        })?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct GetTransitionResponse {
    transitions: Vec<Transition>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Transition {
    id: String,
    name: String,
}
