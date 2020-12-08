use std::fs;

use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use serde::Deserialize;

use crate::workflow::Workflow;

/// This is the top level structure that your `flow.toml` must adhere to to be valid. If config
/// cannot be validated against the structures defined within Config, you'll get an error message
/// right off the bat.
///
/// ## Example
/// ```toml
/// [[workflows]]
/// name = "First Workflow"
/// # Details here
///
/// [[workflows]]
/// name = "Second Workflow"
/// # Details here
///
/// [jira]
/// # JiraConfig here
/// ```
///
/// ## See Also
/// [`Workflow`] for details on defining entries to the `[[workflows]]` array and [`JiraConfig`]
/// for details on defining `[jira]`.
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

/// Details needed to use steps that reference Jira issues.
///
/// ## Example
/// ```TOML
/// [jira]
/// url = "https://mysite.atlassian.net"
/// project = "PRJ"  # where an example issue would be PRJ-123
/// ```
#[derive(Debug, Default, Deserialize)]
pub struct JiraConfig {
    pub url: String,
    pub project: String,
}
