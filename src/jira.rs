use std::fmt;

use crate::state::State;
use color_eyre::eyre::{eyre, Result, WrapErr};
use console::Term;
use dialoguer::{theme::ColorfulTheme, Select};
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

pub async fn select_issue(status: String, mut state: State) -> Result<State> {
    let mut issues = get_issues(status).await?;
    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&issues)
        .default(0)
        .interact_on_opt(&Term::stderr())?;

    match selection {
        Some(index) => {
            let Issue { key, .. } = issues.remove(index);
            println!("User selected item : {}", &key);
            state.selected_issue_key = Some(key);
            Ok(state)
        }
        None => Err(eyre!("No issue selected")),
    }
}

fn get_auth() -> String {
    let token = std::env::var("JIRA_TOKEN").unwrap();
    let email = std::env::var("EMAIL").unwrap();
    format!("Basic {}", base64::encode(format!("{}:{}", email, token)))
}

async fn get_issues(status: String) -> Result<Vec<Issue>> {
    // TODO: Move client into state
    // TODO: Make this URL configurable
    // TODO: Handle this error gracefully
    let auth = get_auth();
    let body = SearchParams {
        jql: format!("status = {}", status),
        fields: vec!["summary"],
    };
    let client = reqwest::Client::new();
    Ok(client
        .post("https://triaxtec.atlassian.net/rest/api/2/search")
        .json(&body)
        .header("Authorization", auth)
        .send()
        .await
        .wrap_err("Could not request issues")?
        .json::<SearchResponse>()
        .await
        .wrap_err("Could not request issues")?
        .issues)
}
