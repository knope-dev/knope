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
