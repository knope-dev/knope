use color_eyre::eyre::{eyre, Result};

use crate::prompt::select;
use crate::state::{Initial, IssueSelected, State};
use std::fmt;

mod jira;

pub enum Issue {
    Jira { key: String, summary: String },
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Issue::Jira { key, summary } => write!(f, "{}: {}", key, summary),
        }
    }
}

pub fn select_issue(status: &str, state: State) -> Result<State> {
    match state {
        State::IssueSelected(..) => Err(eyre!("You've already selected an issue!")),
        State::Initial(Initial { jira_config }) => {
            let issues = jira::get_issues(&jira_config, status)?;
            let issue = select(issues, "Select an Issue")?;
            println!("Selected item : {}", &issue);
            Ok(State::IssueSelected(IssueSelected { jira_config, issue }))
        }
    }
}

pub fn transition_selected_issue(status: &str, state: State) -> Result<State> {
    match state {
        State::Initial(..) => Err(eyre!(
            "No issue selected, try running a SelectIssue step before this one"
        )),
        State::IssueSelected(IssueSelected { jira_config, issue }) => match issue {
            Issue::Jira { key, summary } => {
                jira::transition_issue(&jira_config, &key, status)?;
                println!("{} transitioned to {}", &key, status);
                Ok(State::IssueSelected(IssueSelected {
                    jira_config,
                    issue: Issue::Jira { key, summary },
                }))
            }
        },
    }
}
