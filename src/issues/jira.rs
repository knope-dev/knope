use color_eyre::eyre::{eyre, Result, WrapErr};
use serde::{Deserialize, Serialize};

use crate::app_config::{get_or_prompt_for_email, get_or_prompt_for_jira_token};
use crate::config::Jira;
use crate::issues::Issue;
use http::StatusCode;

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

fn get_auth() -> Result<String> {
    let email = get_or_prompt_for_email()?;
    let token = get_or_prompt_for_jira_token()?;
    Ok(format!(
        "Basic {}",
        base64::encode(format!("{}:{}", email, token))
    ))
}

pub(crate) fn get_issues(jira_config: &Jira, status: &str) -> Result<Vec<Issue>> {
    let auth = get_auth()?;
    let jql = format!("status = {} AND project = {}", status, jira_config.project);
    let url = format!("{}/rest/api/3/search", jira_config.url);
    Ok(ureq::post(&url)
        .set("Authorization", &auth)
        .send_json(ureq::json!({"jql": jql, "fields": ["summary"]}))?
        .into_json::<SearchResponse>()
        .wrap_err("Could not request issues")?
        .issues
        .into_iter()
        .map(|jira_issue| Issue {
            key: jira_issue.key,
            summary: jira_issue.fields.summary,
        })
        .collect())
}

pub(crate) fn transition_issue(jira_config: &Jira, issue_key: &str, status: &str) -> Result<()> {
    let auth = get_auth()?; // TODO: get auth once and store in state
    let url = format!(
        "{}/rest/api/3/issue/{}/transitions",
        jira_config.url, issue_key
    );
    let agent = ureq::Agent::new();
    let response = agent.get(&url).set("Authorization", &auth).call()?;
    if !StatusCode::from_u16(response.status())?.is_success() {
        return Err(eyre!(
            "Received {} when transitioning issue with body {:#?}",
            response.status(),
            response.into_json()?
        ));
    }
    let response = response
        .into_json::<GetTransitionResponse>()
        .wrap_err("Could not decode transitions")?;
    let transition = response
        .transitions
        .into_iter()
        .find(|transition| transition.name == status)
        .ok_or_else(|| eyre!("No matching transition found"))?;
    let response = agent
        .post(&url)
        .set("Authorization", &auth)
        .send_json(ureq::json!({"transition": {"id": transition.id}}))?;
    if !StatusCode::from_u16(response.status())?.is_success() {
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
