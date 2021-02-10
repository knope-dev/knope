use color_eyre::eyre::{eyre, Result};

use crate::prompt::select;
use crate::state::{Initial, IssueSelected, State};
use std::fmt;

mod github;
mod jira;

#[derive(Debug, PartialEq)]
pub(crate) struct Issue {
    pub(crate) key: String,
    pub(crate) summary: String,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.key, self.summary)
    }
}

pub(crate) async fn select_jira_issue(status: &str, state: State) -> Result<State> {
    match state {
        State::IssueSelected(..) => Err(eyre!("You've already selected an issue!")),
        State::Initial(Initial {
            jira_config,
            github_state,
            github_config,
        }) => {
            let jira_config = jira_config.ok_or_else(|| eyre!("Jira is not configured"))?;
            let issues = jira::get_issues(&jira_config, status).await?;
            let issue = select(issues, "Select an Issue")?;
            println!("Selected item : {}", &issue);
            Ok(State::IssueSelected(IssueSelected {
                jira_config: Some(jira_config),
                github_state,
                github_config,
                issue,
            }))
        }
    }
}

pub(crate) async fn select_github_issue(
    labels: Option<Vec<String>>,
    state: State,
) -> Result<State> {
    match state {
        State::IssueSelected(..) => Err(eyre!("You've already selected an issue!")),
        State::Initial(Initial {
            jira_config,
            github_state,
            github_config,
        }) => {
            let (github_config, github_state, issues) =
                github::list_issues(github_config, github_state, labels).await?;
            let issue = select(issues, "Select an Issue")?;
            println!("Selected item : {}", &issue);
            Ok(State::IssueSelected(IssueSelected {
                jira_config,
                github_state,
                github_config,
                issue,
            }))
        }
    }
}

pub(crate) async fn transition_selected_issue(status: &str, state: State) -> Result<State> {
    match state {
        State::Initial(..) => Err(eyre!(
            "No issue selected, try running a SelectIssue step before this one"
        )),
        State::IssueSelected(IssueSelected {
            jira_config,
            github_state,
            github_config,
            issue,
        }) => {
            let jira_config = jira_config.ok_or_else(|| eyre!("Jira is not configured"))?;
            jira::transition_issue(&jira_config, &issue.key, status).await?;
            println!("{} transitioned to {}", &issue.key, status);
            Ok(State::IssueSelected(IssueSelected {
                jira_config: Some(jira_config),
                github_state,
                github_config,
                issue,
            }))
        }
    }
}
