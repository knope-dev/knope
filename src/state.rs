use crate::config::JiraConfig;

pub struct Initial {
    pub jira_config: JiraConfig,
}

pub struct Issue {
    pub key: String,
    pub summary: String,
}

pub struct IssueSelected {
    pub jira_config: JiraConfig,
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
    pub fn new(jira_config: JiraConfig) -> Self {
        State::Initial(Initial { jira_config })
    }
}
