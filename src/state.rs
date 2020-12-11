use crate::config::Jira;
use crate::issues::Issue;

pub struct Initial {
    pub jira_config: Jira,
}

pub struct IssueSelected {
    pub jira_config: Jira,
    pub issue: Issue,
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
    pub(crate) fn new(jira_config: Jira) -> Self {
        State::Initial(Initial { jira_config })
    }
}
