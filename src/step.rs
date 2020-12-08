use std::collections::HashMap;

use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use serde::Deserialize;

use crate::state::State;
use crate::{command, git, jira, semver};

pub(crate) fn run_step(step: Step, state: State) -> Result<State> {
    match step {
        Step::SelectIssue { status } => {
            jira::select_issue(status, state).wrap_err("During SelectIssue")
        }
        Step::TransitionIssue { status } => {
            jira::transition_selected_issue(status, state).wrap_err("During TransitionIssue")
        }
        Step::SwitchBranches => git::switch_branches(state).wrap_err("During SwitchBranches"),
        Step::RebaseBranch { to } => git::rebase_branch(state, to).wrap_err("During MergeBranch"),
        Step::BumpVersion(rule) => semver::bump_version(state, rule).wrap_err("During BumpVersion"),
        Step::Command { command, variables } => {
            command::run_command(state, command, variables).wrap_err("During Command")
        }
    }
}

/// Each variant describes an action you can take using Flow, they are used when defining your
/// [`crate::workflow::Workflow`] via whatever config format is being utilized.
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Step {
    /// Search for Jira issues by status and display the list of them in the terminal.
    /// User is allowed to select one issue which will then change the workflow's state to
    /// [`State::IssueSelected`].
    ///
    /// ## Errors
    /// This step will fail if any of the following are true:
    /// 1. The workflow is already in [`State::IssueSelected`] before it executes.
    /// 2. Flow cannot communicate with the configured Jira URL.
    /// 3. User does not select an issue.
    ///
    /// ## Example
    /// ```toml
    /// # flow.toml
    /// [[workflows]]
    /// name = "Start some work"
    ///     [[workflows.steps]]
    ///     type = "SelectIssue"
    ///     status = "Backlog"
    /// ```
    SelectIssue {
        /// Issues with this status in Jira will be listed for the user to select.
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
        variables: Option<HashMap<String, command::Variable>>,
    },
}
