use crate::workflow::JiraConfig;

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

pub enum State {
    Initial(Initial),
    IssueSelected(IssueSelected),
}

impl State {
    pub fn new(jira_config: JiraConfig) -> Self {
        State::Initial(Initial { jira_config })
    }
}
