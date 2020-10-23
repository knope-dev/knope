use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use serde::Deserialize;
use tokio::fs;

pub async fn load_workflow() -> Result<Config> {
    let contents = fs::read_to_string("config.toml")
        .await
        .wrap_err("Could not find config file.")?;
    toml::from_str(&contents).wrap_err("Failed to parse config file.")
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub workflows: Vec<Workflow>,
    pub projects: Vec<Project>,
}

#[derive(Deserialize, Debug)]
pub struct Workflow {
    pub name: String,
    pub steps: Vec<Step>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Step {
    SelectIssue { status: String },
}

#[derive(Deserialize, Debug)]
pub struct Project {
    jira_key: String,
    directory: String,
}
