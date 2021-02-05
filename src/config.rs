use std::fs;

use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use serde::Deserialize;

use crate::workflow::Workflow;

#[derive(Deserialize, Debug)]
pub struct Config {
    /// The list of defined workflows that are selectable
    pub workflows: Vec<Workflow>,
    /// Optional configuration for Jira
    pub jira: Option<Jira>,
    /// Optional configuration to talk to GitHub
    pub github: Option<GitHub>,
}

impl Config {
    /// Create a Config from a TOML file.
    ///
    /// ## Errors
    /// 1. Provided path is not found
    /// 2. Cannot parse file contents into a Config
    pub fn load(path: &str) -> Result<Self> {
        let contents = fs::read_to_string(path).wrap_err("Could not find config file.")?;
        toml::from_str(&contents).wrap_err("Failed to parse config file.")
    }
}

/// Config required for steps that interact with Jira.
#[derive(Debug, Default, Deserialize)]
pub struct Jira {
    /// The URL to your Atlassian instance running Jira
    pub url: String,
    /// The key of the Jira project to filter on (the prefix of all issues)
    pub project: String,
}

/// Details needed to use steps that interact with GitHub.
#[derive(Debug, Default, Deserialize)]
pub struct GitHub {
    /// The user or organization that owns the `repo`.
    pub owner: String,
    /// The name of the repository in GitHub that this project is utilizing
    pub repo: String,
}
