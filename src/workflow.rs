use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use serde::export::Formatter;
use serde::Deserialize;
use std::fs;

pub fn load_workflow() -> Result<Config> {
    let contents = fs::read_to_string("flow.toml").wrap_err("Could not find config file.")?;
    toml::from_str(&contents).wrap_err("Failed to parse config file.")
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub workflows: Vec<Workflow>,
    pub jira: JiraConfig,
}

#[derive(Deserialize, Debug)]
pub struct Workflow {
    pub name: String,
    pub steps: Vec<Step>,
}

impl std::fmt::Display for Workflow {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.name)
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Step {
    SelectIssue { status: String },
    TransitionIssue { status: String },
    SwitchBranches,
}

#[derive(Debug, Default, Deserialize)]
pub struct JiraConfig {
    pub url: String,
    pub project: String,
}
