use color_eyre::eyre::{Result, WrapErr};
use console::Term;
use dialoguer::{theme::ColorfulTheme, Select};
use dotenv::dotenv;
use serde::export::Formatter;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Debug)]
struct SearchParams {
    jql: &'static str,
    fields: Vec<&'static str>,
}

#[derive(Deserialize, Debug)]
struct IssueFields {
    summary: String,
}

#[derive(Deserialize, Debug)]
struct Issue {
    key: String,
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

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().unwrap();
    dotenv().ok();
    // TODO: Handle this error and print out a useful message about generating a token
    // TODO: store this in keychain instead of env var
    let token = std::env::var("JIRA_TOKEN").unwrap();
    let email = std::env::var("EMAIL").unwrap();
    let jira_auth = format!("Basic {}", base64::encode(format!("{}:{}", email, token)));
    let client = reqwest::Client::new();
    let issues = get_selected(&client, &jira_auth)
        .await
        .wrap_err("Failed to fetch selected issues")?;
    select_issue(issues)
}

fn select_issue(issues: Vec<Issue>) -> Result<()> {
    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&issues)
        .default(0)
        .interact_on_opt(&Term::stderr())?;

    match selection {
        Some(index) => println!("User selected item : {}", issues.get(index).unwrap()),
        None => println!("User did not select anything"),
    }

    Ok(())
}

async fn get_selected(client: &reqwest::Client, auth: &str) -> Result<Vec<Issue>> {
    // TODO: Make this URL configurable
    // TODO: Handle this error gracefully
    let body = SearchParams {
        jql: "status = selected",
        fields: vec!["summary"],
    };
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
