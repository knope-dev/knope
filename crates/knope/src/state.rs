use std::fmt::Debug;

use knope_versioning::{Action, VersionedFile};

use crate::{
    config,
    step::{issues, releases},
};

/// The current state of the workflow. Every [`crate::Step`] has a chance to transform the state.
#[derive(Clone, Debug)]
pub(crate) struct State {
    pub(crate) jira_config: Option<config::Jira>,
    pub(crate) github: GitHub,
    pub(crate) gitea: Gitea,
    pub(crate) gitea_config: Option<config::Gitea>,
    pub(crate) github_config: Option<config::GitHub>,
    pub(crate) issue: Issue,
    pub(crate) packages: Vec<releases::Package>,
    pub(crate) all_versioned_files: Vec<VersionedFile>,
    pub(crate) pending_actions: Vec<Action>,
    pub(crate) all_git_tags: Vec<String>,
    /// If set to true, conventional commits are ignored across all workflows
    pub(crate) ignore_conventional_commits: bool,
}

impl State {
    #[must_use]
    pub(crate) fn new(
        jira_config: Option<config::Jira>,
        github_config: Option<config::GitHub>,
        gitea_config: Option<config::Gitea>,
        packages: Vec<releases::Package>,
        all_versioned_files: Vec<VersionedFile>,
        all_git_tags: Vec<String>,
        ignore_conventional_commits: bool,
    ) -> Self {
        State {
            jira_config,
            gitea: Gitea::New,
            gitea_config,
            github: GitHub::New,
            github_config,
            issue: Issue::Initial,
            packages,
            all_versioned_files,
            all_git_tags,
            pending_actions: Vec::new(),
            ignore_conventional_commits,
        }
    }
}

/// The type of state—an outer enum to make sure that dry-runs are handled appropriately.
#[derive(Clone, Copy, Debug)]
pub(crate) enum RunType<T> {
    /// Signifies that this is a dry run of a workflow. No I/O should happen—just pretend to run the
    /// workflow and output the results.
    DryRun(T),
    /// This is a real run of a workflow, actually do the thing.
    Real(T),
}

impl<T> RunType<T> {
    #[must_use]
    pub(crate) fn of<R>(&self, new_value: R) -> RunType<R> {
        match self {
            RunType::DryRun(_) => RunType::DryRun(new_value),
            RunType::Real(_) => RunType::Real(new_value),
        }
    }

    pub(crate) fn take(self) -> (RunType<()>, T) {
        match self {
            RunType::DryRun(inner) => (RunType::DryRun(()), inner),
            RunType::Real(inner) => (RunType::Real(()), inner),
        }
    }
}

/// Tracks what's been done with respect to issues in this workflow.
#[derive(Clone, Debug)]
pub(crate) enum Issue {
    /// All workflows start here—no issue has been selected yet.
    Initial,
    /// Triggered by [`crate::Step::SelectJiraIssue`] or [`crate::Step::SelectGitHubIssue`],
    /// contains details of the issue you're working against to use for things like transitioning
    /// or creating branches.
    Selected(issues::Issue),
}

#[derive(Clone, Debug)]
pub(crate) enum GitHub {
    New,
    Initialized { token: String, agent: ureq::Agent },
}

#[derive(Clone, Debug)]
pub(crate) enum Gitea {
    New,
    Initialized { token: String, agent: ureq::Agent },
}
