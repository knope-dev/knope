use std::io::Write;
use reqwest::Client;

use crate::{
    config,
    step::{issues, releases},
    workflow::Verbose,
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
    pub(crate) verbose: Verbose,
    pub(crate) client: Option<Client>,
}

impl State {
    #[must_use]
    pub(crate) fn new(
        jira_config: Option<config::Jira>,
        github_config: Option<config::GitHub>,
        gitea_config: Option<config::Gitea>,
        packages: Vec<releases::Package>,
        verbose: Verbose,
    ) -> Self {
        State {
            jira_config,
            gitea: Gitea::New,
            gitea_config,
            github: GitHub::New,
            github_config,
            issue: Issue::Initial,
            packages,
            verbose,
            client: None,
        }
    }

    pub(crate) fn get_client(&mut self) -> Client {
        match self.client.clone() {
            Some(client) => client,
            None => {
                let client = Client::new();
                self.client = Some(client.clone());
                client
            }
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

    pub(crate) fn recompose(state: State, dry_run: Option<Box<dyn Write>>) -> Self {
        if let Some(stdout) = dry_run {
            RunType::DryRun { state, stdout }
        } else {
            RunType::Real(state)
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
    Initialized { token: String },
}

#[derive(Clone, Debug)]
pub(crate) enum Gitea {
    New,
    Initialized { token: String },
}
