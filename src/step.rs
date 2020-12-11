use std::collections::HashMap;

use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use serde::Deserialize;

use crate::state::State;
use crate::{command, git, issues, semver};

pub(crate) async fn run_step(step: Step, state: State) -> Result<State> {
    match step {
        Step::SelectJiraIssue { status } => issues::select_jira_issue(&status, state)
            .await
            .wrap_err("During SelectJiraIssue"),
        Step::SelectGitHubIssue { labels } => issues::select_github_issue(labels, state)
            .await
            .wrap_err("During SelectGitHubIssue"),
        Step::TransitionJiraIssue { status } => issues::transition_selected_issue(&status, state)
            .await
            .wrap_err("During TransitionJiraIssue"),
        Step::SwitchBranches => git::switch_branches(state).wrap_err("During SwitchBranches"),
        Step::RebaseBranch { to } => git::rebase_branch(state, &to).wrap_err("During MergeBranch"),
        Step::BumpVersion(rule) => {
            semver::bump_version(state, &rule).wrap_err("During BumpVersion")
        }
        Step::Command { command, variables } => {
            command::run_command(state, command, variables).wrap_err("During Command")
        }
    }
}

/// Each variant describes an action you can take using Dobby, they are used when defining your
/// [`crate::Workflow`] via whatever config format is being utilized.
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
    /// 2. Dobby cannot communicate with the configured Jira URL.
    /// 3. User does not select an issue.
    /// 4. There is no [`crate::config::Jira`] set.
    ///
    /// ## Example
    /// ```toml
    /// # dobby.toml
    /// [[workflows]]
    /// name = "Start some work"
    ///     [[workflows.steps]]
    ///     type = "SelectIssue"
    ///     status = "Backlog"
    /// ```
    SelectJiraIssue {
        /// Issues with this status in Jira will be listed for the user to select.
        status: String,
    },
    /// Transition a Jira issue to a new status.
    ///
    /// ## Errors
    /// This step will fail when any of the following are true:
    /// 1. The workflow is not yet in [`State::IssueSelected`] ([`Step::SelectIssue`] was not run
    ///     before this step).
    /// 2. Cannot communicate with Jira.
    /// 3. The configured status is invalid for the issue.
    /// 4. The selected issue is a GitHub issue instead of a Jira issue.
    ///
    /// ## Example
    /// ```toml
    /// # dobby.toml
    /// [[workflows]]
    /// name = "Start some work"
    ///     [[workflows.steps]]
    ///     type = "SelectJiraIssue"
    ///     status = "Backlog"
    ///     
    ///     [[workflows.steps]]
    ///     type = "TransitionJiraIssue"
    ///     status = "In Progress"
    /// ```
    TransitionJiraIssue {
        /// The status to transition the current issue to.
        status: String,
    },
    /// Search for GitHub issues by status and display the list of them in the terminal.
    /// User is allowed to select one issue which will then change the workflow's state to
    /// [`State::IssueSelected`].
    ///
    /// ## Errors
    /// This step will fail if any of the following are true:
    /// 1. The workflow is already in [`State::IssueSelected`] before it executes.
    /// 2. Dobby cannot communicate with GitHub.
    /// 4. There is no [`crate::config::GitHub`] set.
    /// 3. User does not select an issue.
    ///
    /// ## Example
    /// ```toml
    /// # dobby.toml
    /// [[workflows]]
    /// name = "Start some work"
    ///     [[workflows.steps]]
    ///     type = "SelectGitHubIssue"
    ///     label = "selected"
    /// ```
    SelectGitHubIssue {
        /// If provided, only issues with this label will be included
        labels: Option<Vec<String>>,
    },
    /// Uses the name of the currently selected issue to checkout an existing or create a new
    /// branch for development. If an existing branch is not found, the user will be prompted to
    /// select an existing local branch to base the new branch off of. Remote branches are not
    /// shown.
    ///
    /// ## Errors
    /// This step fails if any of the following are true.
    /// 1. Workflow is not in [`State::IssueSelected`], as [`Step::SelectIssue`] was not run before
    ///     this step.
    /// 2. Current directory is not a Git repository
    ///
    /// ## Example
    /// ```toml
    /// # dobby.toml
    /// [[workflows]]
    /// name = "Start some work"
    ///     [[workflows.steps]]
    ///     type = "SelectIssue"
    ///     status = "Backlog"
    ///     
    ///     [[workflows.steps]]
    ///     type = "SwitchBranches"
    /// ```
    SwitchBranches,
    /// Rebase the current branch onto the branch defined by `to`.
    ///
    /// ## Errors
    /// Fails if any of the following are true:
    /// 1. The current directory is not a Git repository.
    /// 2. The `to` branch cannot be found locally (does not check remotes).
    /// 3. The repo is not on the tip of a branch (e.g. detached HEAD)
    /// 4. Rebase fails (e.g. not a clean working tree)
    ///
    /// ## Example
    /// ```toml
    /// # dobby.toml
    /// [[workflows]]
    /// name = "Finish some work"
    ///     [[workflows.steps]]
    ///     type = "RebaseBranch"
    ///     to = "main"
    /// ```
    RebaseBranch {
        /// The branch to rebase onto.
        to: String,
    },
    /// Bump the version of the project in any supported formats found using a
    /// [Semantic Versioning](https://semver.org) rule.
    ///
    /// ## Supported Formats
    /// These are the types of files that this step knows how to search for a semantic version and
    /// bump:
    /// 1. Cargo.toml in the current directory.
    ///
    /// ## Rules
    /// Details about the rules that can be provided to this step can be found in [`semver::Rule`].
    ///
    /// ## Errors
    /// This step will fail if any of the following are true:
    /// 1. A malformed version string is found while attempting to bump.
    ///
    /// ## Example
    /// ```toml
    /// [[workflows.steps]]
    /// type = "BumpVersion"
    /// rule = "Pre"
    /// value = "rc"
    /// ```
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
