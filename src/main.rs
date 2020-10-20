use async_std::task;
use color_eyre::eyre::{Result, WrapErr};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
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

#[derive(Deserialize, Debug)]
struct SearchResponse {
    issues: Vec<Issue>,
}

fn main() {
    color_eyre::install().unwrap();
    dotenv().ok();
    // TODO: Handle this error and print out a useful message about generating a token
    // TODO: store this in keychain instead of env var
    let token = std::env::var("JIRA_TOKEN").unwrap();
    let email = std::env::var("EMAIL").unwrap();
    let jira_auth = format!("Basic {}", base64::encode(format!("{}:{}", email, token)));
    task::block_on(get_selected(&jira_auth))
}

async fn get_selected(auth: &str) {
    // TODO: Make this URL configurable
    // TODO: Handle this error gracefully
    let response: SearchResponse = surf::post("https://triaxtec.atlassian.net/rest/api/2/search")
        .body(
            surf::Body::from_json(&SearchParams {
                jql: "status = selected",
                fields: vec!["summary"],
            })
            .unwrap(),
        )
        .header("Authorization", auth)
        .recv_json()
        .await
        .unwrap();
    println!("{:#?}", response);
}
