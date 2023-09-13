use std::collections::HashMap;

use log::error;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    command, git, issues, prompt, releases, releases::semver::Label, state::RunType,
    workflow::Verbose,
};

/// Each variant describes an action you can take using knope, they are used when defining your
/// [`crate::Workflow`] via whatever config format is being utilized.
#[derive(Deserialize, Debug, Serialize)]
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
    BumpVersion(releases::Rule),
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
    PrepareRelease(PrepareRelease),
    /// This will create a new release on GitHub using the current project version.
    ///
    /// Requires that GitHub details be configured.
    Release,
    /// Create a new change file to be included in the next release.
    ///
    /// This step is interactive and will prompt the user for the information needed to create the
    /// change file. Do not try to run in a non-interactive environment.
    CreateChangeFile,
}

impl Step {
    pub(crate) fn run(self, run_type: RunType, verbose: Verbose) -> Result<RunType, Error> {
        Ok(match self {
            Step::SelectJiraIssue { status } => issues::jira::select_issue(&status, run_type)?,
            Step::TransitionJiraIssue { status } => {
                issues::jira::transition_issue(&status, run_type)?
            }
            Step::SelectGitHubIssue { labels } => {
                issues::github::select_issue(labels.as_deref(), run_type)?
            }
            Step::SwitchBranches => git::switch_branches(run_type)?,
            Step::RebaseBranch { to } => git::rebase_branch(&to, run_type)?,
            Step::BumpVersion(rule) => releases::bump_version(run_type, &rule, verbose)?,
            Step::Command { command, variables } => {
                command::run_command(run_type, command, variables, verbose)?
            }
            Step::PrepareRelease(prepare_release) => {
                releases::prepare_release(run_type, &prepare_release, verbose)?
            }
            Step::SelectIssueFromBranch => git::select_issue_from_current_branch(run_type)?,
            Step::Release => releases::release(run_type, verbose)?,
            Step::CreateChangeFile => releases::create_change_file(run_type)?,
        })
    }

    /// Set `prerelease_label` if `self` is `PrepareRelease`.
    pub(crate) fn set_prerelease_label(&mut self, prerelease_label: &str) {
        if let Step::PrepareRelease(prepare_release) = self {
            prepare_release.prerelease_label = Some(Label::from(prerelease_label));
        }
    }
}

#[derive(Debug, Error, Diagnostic)]
pub(super) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Release(#[from] releases::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Prompt(#[from] prompt::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    JiraIssue(#[from] issues::jira::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    GitHubIssue(#[from] issues::github::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(#[from] git::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    ChangeSet(#[from] releases::changesets::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Command(#[from] command::Error),
}

/// The inner content of a [`Step::PrepareRelease`] step.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct PrepareRelease {
    /// If set, the user wants to create a pre-release version using the selected label.
    pub(crate) prerelease_label: Option<Label>,
}
