use color_eyre::eyre::{eyre, Result, WrapErr};
use serde::{Deserialize, Serialize};

use crate::app_config::{get_or_prompt_for_email, get_or_prompt_for_jira_token};
use crate::config::Jira;
use crate::issues::Issue;

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

pub(crate) async fn get_issues(jira_config: &Jira, status: &str) -> Result<Vec<Issue>> {
    let auth = get_auth()?;
    let jql = format!("status = {} AND project = {}", status, jira_config.project);
    let url = format!("{}/rest/api/3/search", jira_config.url);
    Ok(reqwest::Client::new()
        .post(&url)
        .header("Authorization", &auth)
        .json(&serde_json::json!({"jql": jql, "fields": ["summary"]}))
        .send()
        .await?
        .json::<SearchResponse>()
        .await
        .wrap_err("Could not request issues")?
        .issues
        .into_iter()
        .map(|jira_issue| Issue::Jira {
            key: jira_issue.key,
            summary: jira_issue.fields.summary,
        })
        .collect())
}

pub(crate) async fn transition_issue(
    jira_config: &Jira,
    issue_key: &str,
    status: &str,
) -> Result<()> {
    let auth = get_auth()?; // TODO: get auth once and store in state
    let url = format!(
        "{}/rest/api/3/issue/{}/transitions",
        jira_config.url, issue_key
    );
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Authorization", &auth)
        .send()
        .await?;
    if response.error_for_status_ref().is_err() {
        return Err(eyre!(
            "Received {} when transitioning issue with body {:#?}",
            response.status(),
            response.json().await?
        ));
    }
    let response = response
        .json::<GetTransitionResponse>()
        .await
        .wrap_err("Could not decode transitions")?;
    let transition = response
        .transitions
        .into_iter()
        .find(|transition| transition.name == status)
        .ok_or_else(|| eyre!("No matching transition found"))?;
    let response = client
        .post(&url)
        .header("Authorization", &auth)
        .json(&serde_json::json!({"transition": {"id": transition.id}}))
        .send()
        .await?;
    if response.error_for_status_ref().is_err() {
        return Err(eyre!(
            "Received {} when transitioning issue with body {:#?}",
            response.status(),
            response.json().await?
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
