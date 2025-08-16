use std::borrow::Cow;

use indexmap::IndexMap;
use knope_config::{Template, Variable};
use knope_versioning::semver::Label;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::debug;

use crate::{
    integrations::git,
    prompt,
    state::{RunType, State},
};

pub mod command;
pub(crate) mod create_change_file;
mod create_pull_request;
pub mod issues;
pub mod releases;

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
    /// Search for Gitea issues by status and display the list of them in the terminal.
    /// User is allowed to select one issue which will then change the workflow's state to
    /// [`Issue::Selected`].
    SelectGiteaIssue {
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
    BumpVersion(Rule),
    /// Run a command in your current shell after optionally replacing some variables.
    Command {
        /// The command to run, with any variable keys you wish to replace.
        command: String,
        /// A map of value-to-replace to [Variable][`crate::command::Variable`] to replace
        /// it with.
        variables: Option<IndexMap<Cow<'static, str>, Variable>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        /// Whether to run the command in the platform's shell or not
        shell: Option<bool>,
    },
    /// This will look through all commits since the last tag and parse any
    /// [Stable Commits](https://www.conventionalcommits.org/en/v1.0.0/) it finds. It will
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
    CreatePullRequest {
        base: String,
        title: Template,
        body: Template,
    },
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(tag = "rule")]
pub(crate) enum Rule {
    Major,
    Minor,
    Patch,
    Pre { label: Label },
    Release,
}

impl From<Rule> for knope_versioning::semver::Rule {
    fn from(value: Rule) -> Self {
        use knope_versioning::semver::{
            Rule::{Pre, Release, Stable},
            StableRule::{Major, Minor, Patch},
        };
        match value {
            Rule::Major => Stable(Major),
            Rule::Minor => Stable(Minor),
            Rule::Patch => Stable(Patch),
            Rule::Pre { label } => Pre {
                label,
                stable_rule: Patch,
            },
            Rule::Release => Release,
        }
    }
}

impl Step {
    pub(crate) fn run(self, state: RunType<State>) -> Result<RunType<State>, Error> {
        debug!("Running step {self:?}");
        Ok(match self {
            Step::SelectJiraIssue { status } => issues::jira::select_issue(&status, state)?,
            Step::TransitionJiraIssue { status } => issues::jira::transition_issue(&status, state)?,
            Step::SelectGitHubIssue { labels } => {
                issues::github::select_issue(labels.as_deref(), state)?
            }
            Step::SelectGiteaIssue { labels } => {
                issues::gitea::select_issue(labels.as_deref(), state)?
            }
            Step::SwitchBranches => git::switch_branches(state)?,
            Step::RebaseBranch { to } => {
                git::rebase_branch(&state.of(to))?;
                state
            }
            Step::BumpVersion(rule) => releases::bump_version(state, &rule.into())?,
            Step::Command {
                command,
                variables,
                shell,
            } => command::run_command(state, command, shell.is_some_and(|it| it), variables)?,
            Step::PrepareRelease(prepare_release) => {
                releases::prepare_release(state, &prepare_release)?
            }
            Step::SelectIssueFromBranch => git::select_issue_from_current_branch(state)?,
            Step::Release => releases::release(state)?,
            Step::CreateChangeFile => create_change_file::run(state)?,
            Step::CreatePullRequest { base, title, body } => {
                create_pull_request::run(&base, &title, &body, state)?
            }
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
    GiteaIssue(#[from] issues::gitea::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(#[from] git::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    ChangeSet(#[from] create_change_file::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Command(#[from] command::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    CreatePullRequest(#[from] create_pull_request::Error),
}

/// The inner content of a [`Step::PrepareRelease`] step.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct PrepareRelease {
    /// If set, the user wants to create a pre-release version using the selected label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) prerelease_label: Option<Label>,
    /// Should this step continue if there are no changes to release? If not, it causes an error.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) allow_empty: bool,
    /// If set to true, conventional commits are ignored
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) ignore_conventional_commits: bool,
}
