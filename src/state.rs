use crate::config;
use crate::issues::Issue;
use octocrab::Octocrab;

pub struct Initial {
    pub(crate) jira_config: Option<config::Jira>,
    pub(crate) github_state: GitHub,
    pub(crate) github_config: Option<config::GitHub>,
}

pub struct IssueSelected {
    pub(crate) jira_config: Option<config::Jira>,
    pub(crate) github_state: GitHub,
    pub(crate) github_config: Option<config::GitHub>,
    pub(crate) issue: Issue,
}

/// The current state of the workflow. All workflows start in `Initial` state and can be transitioned
/// to other States using certain [`crate::Step`]s.
pub enum State {
    /// The starting state for all workflows, contains some config information only.
    Initial(Initial),
    /// Triggered by [`crate::Step::SelectIssue`], contains details of the Jira issue you're working
    /// against.
    IssueSelected(IssueSelected),
}

impl State {
    #[must_use]
    pub(crate) fn new(
        jira_config: Option<config::Jira>,
        github_config: Option<config::GitHub>,
    ) -> Self {
        State::Initial(Initial {
            jira_config,
            github_state: GitHub::New,
            github_config,
        })
    }
}

pub enum GitHub {
    New,
    Initialized { octocrab: Octocrab },
}
