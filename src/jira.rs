use std::fmt;

use crate::config::Jira;
use crate::prompt::select;
use crate::state;
use crate::state::{Initial, IssueSelected, State};
use color_eyre::eyre::{eyre, Result, WrapErr};
use serde::export::Formatter;
use serde::{Deserialize, Serialize};

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
pub struct Issue {
    pub key: String,
    fields: IssueFields,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.key, self.fields.summary)
    }
}

#[derive(Deserialize, Debug)]
struct SearchResponse {
    issues: Vec<Issue>,
}

pub fn select_issue(status: &str, state: State) -> Result<State> {
    match state {
        State::IssueSelected(..) => Err(eyre!("You've already selected an issue!")),
        State::Initial(Initial { jira_config }) => {
            let issues = get_issues(&jira_config, status)?;
            let issue = select(issues, "Select an Issue")?;
            println!("Selected item : {}", &issue.key);
            Ok(State::IssueSelected(IssueSelected {
                jira_config,
                issue: state::Issue {
                    key: issue.key,
                    summary: issue.fields.summary,
                },
            }))
        }
    }
}

fn get_auth() -> Result<String> {
    // TODO: Handle this error and print out a useful message about generating a token (https://id.atlassian.com/manage-profile/security/api-tokens)
    // TODO: store this in keychain instead of env var
    let token = std::env::var("JIRA_TOKEN")
        .wrap_err("You must have the JIRA_TOKEN variable set in .env or an environment variable")?;
    let email = std::env::var("EMAIL")
        .wrap_err("You must have the EMAIL variable set in .env or an environment variable")?;
    Ok(format!(
        "Basic {}",
        base64::encode(format!("{}:{}", email, token))
    ))
}

fn get_issues(jira_config: &Jira, status: &str) -> Result<Vec<Issue>> {
    let auth = get_auth()?;
    let jql = format!("status = {} AND project = {}", status, jira_config.project);
    let url = format!("{}/rest/api/3/search", jira_config.url);
    Ok(ureq::post(&url)
        .set("Authorization", &auth)
        .send_json(serde_json::json!({"jql": jql, "fields": ["summary"]}))
        .into_json_deserialize::<SearchResponse>()
        .wrap_err("Could not request issues")?
        .issues)
}

fn transition_issue(jira_config: &Jira, issue_key: &str, status: &str) -> Result<()> {
    let auth = get_auth()?; // TODO: get auth once and store in state
    let url = format!(
        "{}/rest/api/3/issue/{}/transitions",
        jira_config.url, issue_key
    );
    let response = ureq::get(&url).set("Authorization", &auth).call();
    if response.error() {
        return Err(eyre!(
            "Received {} when transitioning issue with body {:#?}",
            response.status(),
            response.into_json()?
        ));
    }
    let response = response
        .into_json_deserialize::<GetTransitionResponse>()
        .wrap_err("Could not decode transitions")?;
    let transition = response
        .transitions
        .into_iter()
        .find(|transition| transition.name == status)
        .ok_or_else(|| eyre!("No matching transition found"))?;
    let response = ureq::post(&url)
        .set("Authorization", &auth)
        .send_json(serde_json::json!({"transition": {"id": transition.id}}));
    if response.error() {
        return Err(eyre!(
            "Received {} when transitioning issue with body {:#?}",
            response.status(),
            response.into_json()?
        ));
    }
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

pub fn transition_selected_issue(status: &str, state: State) -> Result<State> {
    match state {
        State::Initial(..) => Err(eyre!(
            "No issue selected, try running a SelectIssue step before this one"
        )),
        State::IssueSelected(IssueSelected { jira_config, issue }) => {
            transition_issue(&jira_config, &issue.key, status)?;
            println!("{} transitioned to {}", &issue.key, status);
            Ok(State::IssueSelected(IssueSelected { jira_config, issue }))
        }
    }
}
