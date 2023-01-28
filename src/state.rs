use std::io::Write;

use crate::{config, issues, releases, releases::semver::Version};

/// The current state of the workflow. Every [`crate::Step`] has a chance to transform the state.
#[derive(Clone, Debug)]
pub(crate) struct State {
    pub(crate) jira_config: Option<config::Jira>,
    pub(crate) github: GitHub,
    pub(crate) github_config: Option<config::GitHub>,
    pub(crate) issue: Issue,
    /// All of the releases that have been prepared in the current workflow.
    pub(crate) releases: Vec<Release>,
    pub(crate) packages: Vec<releases::Package>,
}

impl State {
    #[must_use]
    pub(crate) fn new(
        jira_config: Option<config::Jira>,
        github_config: Option<config::GitHub>,
        packages: Vec<releases::Package>,
    ) -> Self {
        State {
            jira_config,
            github: GitHub::New,
            github_config,
            issue: Issue::Initial,
            releases: Vec::with_capacity(packages.len()),
            packages,
        }
    }
}

/// The type of state—an outer enum to make sure that dry-runs are handled appropriately.
pub(crate) enum RunType {
    /// Signifies that this is a dry run of a workflow. No I/O should happen—just pretend to run the
    /// workflow and output the results.
    DryRun {
        state: State,
        stdout: Box<dyn Write>,
    },
    /// This is a real run of a workflow, actually do the thing.
    Real(State),
}

impl RunType {
    pub(crate) fn decompose(self) -> (State, Option<Box<dyn Write>>) {
        match self {
            RunType::DryRun { state, stdout } => (state, Some(stdout)),
            RunType::Real(state) => (state, None),
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

/// Tracks what's been done with respect to releases in this workflow.
#[derive(Clone, Debug)]
pub(crate) enum Release {
    /// Triggered by [`crate::Step::BumpVersion`].
    Bumped {
        version: Version,
        package_name: Option<String>,
    },
    /// Triggered by [`crate::Step::PrepareRelease`]. Contains the generated release notes and new
    /// version number.
    Prepared(releases::Release),
}

#[derive(Clone, Debug)]
pub(crate) enum GitHub {
    New,
    Initialized { token: String },
}
