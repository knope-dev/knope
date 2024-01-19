use base64::{prelude::BASE64_STANDARD as base64, Engine};
use miette::Diagnostic;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::Issue;
use crate::{
    app_config,
    app_config::{get_or_prompt_for_email, get_or_prompt_for_jira_token},
    config::Jira,
    prompt,
    prompt::select,
    state,
    state::RunType,
};

pub(crate) async fn select_issue(status: &str, run_type: RunType) -> Result<RunType, Error> {
    let (mut state, dry_run_stdout) = run_type.decompose();
    let jira_config = state.jira_config.as_ref().ok_or(Error::NotConfigured)?;

    if let Some(mut stdout) = dry_run_stdout {
        writeln!(
            stdout,
            "Would query configured Jira instance for issues with status {status}"
        )?;
        writeln!(
            stdout,
            "Would prompt user to select an issue and move workflow to IssueSelected state."
        )?;
        state.issue = state::Issue::Selected(Issue {
            key: "FAKE-123".to_string(),
            summary: "Test issue".to_string(),
        });
        return Ok(RunType::DryRun { state, stdout });
    }

    let client = state.get_client();
    let issues = get_issues(client, jira_config, status).await?;
    let issue = select(issues, "Select an Issue")?;
    println!("Selected item : {}", &issue);
    state.issue = state::Issue::Selected(issue);
    Ok(RunType::Real(state))
}

pub(crate) async fn transition_issue(status: &str, run_type: RunType) -> Result<RunType, Error> {
    let (mut state, dry_run_stdout) = run_type.decompose();
    let issue = match &state.issue {
        state::Issue::Selected(issue) => issue,
        state::Issue::Initial => return Err(Error::NoIssueSelected),
    };
    let jira_config = state.jira_config.as_ref().ok_or(Error::NotConfigured)?;

    if let Some(mut stdout) = dry_run_stdout {
        writeln!(
            stdout,
            "Would transition currently selected issue to status {status}"
        )?;
        return Ok(RunType::DryRun { state, stdout });
    }

    run_transition(state.get_client(), jira_config, &issue.key, status).await?;
    let key = &issue.key;
    println!("{key} transitioned to {status}");
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
    #[error("Unable to write to stdout: {0}")]
    Stdout(#[from] std::io::Error),
    #[error("Problem communicating with Jira while {activity}: {inner}")]
    Api {
        activity: &'static str,
        #[source]
        inner: Box<reqwest::Error>,
    },
    #[error("The specified transition name was not found in the Jira project")]
    #[diagnostic(
        code(issues::jira::transition),
        help("The `transition` field in TransitionJiraIssue must correspond to a valid transition in the Jira project"),
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

#[derive(Serialize, Debug)]
struct SearchParams {
    jql: String,
    fields: Vec<&'static str>,
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

pub(crate) async fn get_issues(client: Client, jira_config: &Jira, status: &str) -> Result<Vec<Issue>, Error> {
    let auth = get_auth()?;
    let project = &jira_config.project;
    let jql = format!("status = {status} AND project = {project}");
    let url = format!("{}/rest/api/3/search", jira_config.url);
    Ok(client.post(&url)
        .set("Authorization", &auth)
        .send_json(json!({"jql": jql, "fields": ["summary"]}))
        .map_err(|inner| Error::Api {
            inner: Box::new(inner),
            activity: "querying for issues",
        })?
        .into_json::<SearchResponse>()?
        .issues
        .into_iter()
        .map(|jira_issue| Issue {
            key: jira_issue.key,
            summary: jira_issue.fields.summary,
        })
        .collect())
}

async fn run_transition(client: Client, jira_config: &Jira, issue_key: &str, status: &str) -> Result<(), Error> {
    let auth = get_auth()?; // TODO: get auth once and store in state
    let base_url = &jira_config.url;
    let url = format!("{base_url}/rest/api/3/issue/{issue_key}/transitions",);
    let response = client
        .get(&url)
        .set("Authorization", &auth)
        .call()
        .map_err(|inner| Error::Api {
            inner: Box::new(inner),
            activity: "getting transitions",
        })?;
    let response = response.into_json::<GetTransitionResponse>()?;
    let transition = response
        .transitions
        .into_iter()
        .find(|transition| transition.name == status)
        .ok_or(Error::Transition)?;
    let _response = client
        .post(&url)
        .set("Authorization", &auth)
        .send_json(json!({"transition": {"id": transition.id}}))
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

#[derive(Debug, Serialize)]
struct PostTransitionBody {
    transition: Transition,
}
