use crate::config;
use crate::issues;
use crate::releases;

/// The current state of the workflow. Every [`crate::Step`] has a chance to transform the state.
pub(crate) struct State {
    pub(crate) jira_config: Option<config::Jira>,
    pub(crate) github: GitHub,
    pub(crate) github_config: Option<config::GitHub>,
    pub(crate) issue: Issue,
    pub(crate) release: Release,
}

impl State {
    #[must_use]
    pub(crate) fn new(
        jira_config: Option<config::Jira>,
        github_config: Option<config::GitHub>,
    ) -> Self {
        State {
            jira_config,
            github: GitHub::New,
            github_config,
            issue: Issue::Initial,
            release: Release::Initial,
        }
    }
}

/// Tracks what's been done with respect to issues in this workflow.
pub(crate) enum Issue {
    /// All workflows start here—no issue has been selected yet.
    Initial,
    /// Triggered by [`crate::Step::SelectJiraIssue`] or [`crate::Step::SelectGitHubIssue`],
    /// contains details of the issue you're working against to use for things like transitioning
    /// or creating branches.
    Selected(issues::Issue),
}

/// Tracks what's been done with respect to releases in this workflow.
pub(crate) enum Release {
    /// All workflows start here—no release has been created yet.
    Initial,
    /// Triggered by [`crate::Step::BumpVersion`].
    Bumped(semver::Version),
    /// Triggered by [`crate::Step::PrepareRelease`]. Contains the generated release notes and new
    /// version number.
    Prepared(releases::Release),
}

pub(crate) enum GitHub {
    New,
    Initialized { token: String },
}
