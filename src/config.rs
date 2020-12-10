use std::fs;

use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use serde::Deserialize;

use crate::workflow::Workflow;

/// This is the top level structure that your `dobby.toml` must adhere to to be valid. If config
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
    /// The list of defined workflows that are selectable
    pub workflows: Vec<Workflow>,
    /// Configuration for Jira
    pub jira: Jira,
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

/// Details needed to use steps that reference Jira issues.
///
/// ## Example
/// ```TOML
/// [jira]
/// url = "https://mysite.atlassian.net"
/// project = "PRJ"  # where an example issue would be PRJ-123
/// ```
#[derive(Debug, Default, Deserialize)]
pub struct Jira {
    /// The URL to your Atlassian instance running Jira
    pub url: String,
    /// The key of the Jira project to filter on (the prefix of all issues)
    pub project: String,
}
