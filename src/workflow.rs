use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use serde::export::Formatter;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub workflows: Vec<Workflow>,
    pub jira: JiraConfig,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let contents = fs::read_to_string(path).wrap_err("Could not find config file.")?;
        toml::from_str(&contents).wrap_err("Failed to parse config file.")
    }
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
    RebaseBranch { to: String },
}

#[derive(Debug, Default, Deserialize)]
pub struct JiraConfig {
    pub url: String,
    pub project: String,
}
