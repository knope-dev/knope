use std::collections::HashMap;

use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use serde::Deserialize;

use crate::state::State;
use crate::{command, git, issues, releases, semver};

pub(crate) fn run_step(step: Step, state: State) -> Result<State> {
    match step {
        Step::SelectJiraIssue { status } => {
            issues::select_jira_issue(&status, state).wrap_err("During SelectJiraIssue")
        }
        Step::SelectGitHubIssue { labels } => {
            issues::select_github_issue(labels.as_ref(), state).wrap_err("During SelectGitHubIssue")
        }
        Step::TransitionJiraIssue { status } => {
            issues::transition_selected_issue(&status, state).wrap_err("During TransitionJiraIssue")
        }
        Step::SwitchBranches => git::switch_branches(state).wrap_err("During SwitchBranches"),
        Step::RebaseBranch { to } => git::rebase_branch(state, &to).wrap_err("During MergeBranch"),
        Step::BumpVersion(rule) => semver::bump_version(state, rule).wrap_err("During BumpVersion"),
        Step::Command { command, variables } => {
            command::run_command(state, command, variables).wrap_err("During Command")
        }
        Step::PrepareRelease {
            changelog_path,
            prerelease_label,
        } => releases::prepare_release(state, &changelog_path, prerelease_label)
            .wrap_err("During PrepareRelease"),
        Step::SelectIssueFromBranch => {
            git::select_issue_from_current_branch(state).wrap_err("During SelectIssueFromBranch")
        }
        Step::Release => releases::release(state).wrap_err("During Release"),
    }
}

/// Each variant describes an action you can take using Dobby, they are used when defining your
/// [`crate::Workflow`] via whatever config format is being utilized.
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub(crate) enum Step {
    /// Search for Jira issues by status and display the list of them in the terminal.
    /// User is allowed to select one issue which will then change the workflow's state to
    /// [`State::IssueSelected`].
    SelectJiraIssue {
        /// Issues with this status in Jira will be listed for the user to select.
        status: String,
    },
    /// Transition a Jira issue to a new status.
    TransitionJiraIssue {
        /// The status to transition the current issue to.
        status: String,
    },
    /// Search for GitHub issues by status and display the list of them in the terminal.
    /// User is allowed to select one issue which will then change the workflow's state to
    /// [`State::IssueSelected`].
    SelectGitHubIssue {
        /// If provided, only issues with this label will be included
        labels: Option<Vec<String>>,
    },
    /// Attempt to parse issue info from the current branch name and change the workflow's state to
    /// [`State::IssueSelected`].
    SelectIssueFromBranch,
    /// Uses the name of the currently selected issue to checkout an existing or create a new
    /// branch for development. If an existing branch is not found, the user will be prompted to
    /// select an existing local branch to base the new branch off of. Remote branches are not
    /// shown.
    SwitchBranches,
    /// Rebase the current branch onto the branch defined by `to`.
    RebaseBranch {
        /// The branch to rebase onto.
        to: String,
    },
    /// Bump the version of the project in any supported formats found using a
    /// [Semantic Versioning](https://semver.org) rule.
    BumpVersion(crate::semver::Rule),
    /// Run a command in your current shell after optionally replacing some variables.
    Command {
        /// The command to run, with any variable keys you wish to replace.
        command: String,
        /// A map of value-to-replace to [Variable][`crate::command::Variable`] to replace
        /// it with.
        variables: Option<HashMap<String, command::Variable>>,
    },
    /// This will look through all commits since the last tag and parse any
    /// [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) it finds. It will
    /// then bump the project version (depending on the rule determined from the commits) and add
    /// a new Changelog entry using the [Keep A Changelog](https://keepachangelog.com/en/1.0.0/)
    /// format.
    PrepareRelease {
        #[serde(default = "default_changelog")]
        changelog_path: String,
        /// If set, the user wants to create a pre-release version using the selected label.
        prerelease_label: Option<String>,
    },
    /// This will create a new release on GitHub using the current project version.
    ///
    /// Requires that GitHub details be configured.
    Release,
}

fn default_changelog() -> String {
    "CHANGELOG.md".to_string()
}
