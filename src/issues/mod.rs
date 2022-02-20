use std::fmt;

use color_eyre::eyre::{eyre, Result};

use crate::prompt::select;
use crate::state::{self, State};

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

pub(crate) fn select_jira_issue(status: &str, state: State) -> Result<State> {
    match state.issue {
        state::Issue::Selected(..) => Err(eyre!("You've already selected an issue!")),
        state::Issue::Initial => {
            let jira_config = state
                .jira_config
                .ok_or_else(|| eyre!("Jira is not configured"))?;
            let issues = jira::get_issues(&jira_config, status)?;
            let issue = select(issues, "Select an Issue")?;
            println!("Selected item : {}", &issue);
            Ok(State {
                jira_config: Some(jira_config),
                github: state.github,
                github_config: state.github_config,
                issue: state::Issue::Selected(issue),
                release: state.release,
            })
        }
    }
}

pub(crate) fn select_github_issue(labels: Option<&Vec<String>>, state: State) -> Result<State> {
    match state.issue {
        state::Issue::Selected(..) => Err(eyre!("You've already selected an issue!")),
        state::Issue::Initial => {
            let (github_config, github, issues) =
                github::list_issues(state.github_config, state.github, labels)?;
            let issue = select(issues, "Select an Issue")?;
            println!("Selected item : {}", &issue);
            Ok(State {
                jira_config: state.jira_config,
                github,
                github_config,
                issue: state::Issue::Selected(issue),
                release: state.release,
            })
        }
    }
}

pub(crate) fn transition_selected_issue(status: &str, state: State) -> Result<State> {
    match &state.issue {
        state::Issue::Selected(issue) => {
            let jira_config = state
                .jira_config
                .as_ref()
                .ok_or_else(|| eyre!("Jira is not configured"))?;
            jira::transition_issue(jira_config, &issue.key, status)?;
            println!("{} transitioned to {}", &issue.key, status);
            Ok(state)
        }
        state::Issue::Initial => Err(eyre!(
            "No issue selected, try running a SelectIssue step before this one"
        )),
    }
}
