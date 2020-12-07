use std::collections::HashMap;
use std::fs;

use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use serde::export::Formatter;
use serde::Deserialize;

use crate::command::Variable;

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

/// Each variant describes an action you can take using Flow, they are used when defining your
/// [`Workflow`] via whatever config format is being utilized.
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Step {
    /// Search for Jira issues by status and display the list of them in the terminal.
    /// User is allowed to select one issue which will then change the workflow's state to
    /// [IssueSelected][`crate::State::IssueSelected`].
    SelectIssue {
        status: String,
    },
    TransitionIssue {
        status: String,
    },
    SwitchBranches,
    RebaseBranch {
        to: String,
    },
    BumpVersion(crate::semver::Rule),
    /// Run a command in your current shell after optionally replacing some variables.
    ///
    /// ## Example
    /// If the current version for your project is "1.0.0", the following workflow step will run
    /// `git tag v.1.0.0` in your current shell.
    ///
    /// ```toml
    /// [[workflows.steps]]
    /// type = "Command"
    /// command = "git tag v.version"
    /// variables = {"version" = "Version"}
    /// ```
    ///
    /// Note that the key ("version" in the example) is completely up to you, make it whatever you
    /// like, but if it's not found in the command string it won't be substituted correctly.
    Command {
        /// The command to run, with any variable keys you wish to replace.
        command: String,
        /// A map of value-to-replace to [Variable][`crate::command::Variable`] to replace
        /// it with.
        variables: Option<HashMap<String, Variable>>,
    },
}

#[derive(Debug, Default, Deserialize)]
pub struct JiraConfig {
    pub url: String,
    pub project: String,
}
