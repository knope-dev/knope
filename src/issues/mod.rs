use std::fmt;

use crate::prompt::select;
use crate::state::{self, RunType, State};
use crate::step::StepError;

mod github;
mod jira;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Issue {
    pub(crate) key: String,
    pub(crate) summary: String,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.key, self.summary)
    }
}

pub(super) fn select_jira_issue(status: &str, run_type: RunType) -> Result<RunType, StepError> {
    let (mut state, dry_run_stdout) = run_type.decompose();
    let jira_config = state
        .jira_config
        .as_ref()
        .ok_or(StepError::JiraNotConfigured)?;

    if let Some(mut stdout) = dry_run_stdout {
        writeln!(
            stdout,
            "Would query configured Jira instance for issues with status {status}"
        )?;
        writeln!(
            stdout,
            "Would prompt user to select an issue and move workflow to IssueSelected state."
        )?;
        state.issue = state::Issue::Selected(Issue {
            key: "FAKE-123".to_string(),
            summary: "Test issue".to_string(),
        });
        return Ok(RunType::DryRun { state, stdout });
    }

    let issues = jira::get_issues(jira_config, status)?;
    let issue = select(issues, "Select an Issue")?;
    println!("Selected item : {}", &issue);
    state.issue = state::Issue::Selected(issue);
    Ok(RunType::Real(state))
}

pub(super) fn select_github_issue(
    labels: Option<&[String]>,
    run_type: RunType,
) -> Result<RunType, StepError> {
    match run_type {
        RunType::DryRun {
            mut state,
            mut stdout,
        } => {
            if state.github_config.is_none() {
                return Err(StepError::GitHubNotConfigured);
            }
            if let Some(labels) = labels {
                writeln!(
                    stdout,
                    "Would query configured GitHub instance for issues with labels {}",
                    labels.join(", ")
                )?;
            } else {
                writeln!(
                    stdout,
                    "Would query configured GitHub instance for issues with any labels"
                )?;
            }
            writeln!(
                stdout,
                "Would prompt user to select an issue and move workflow to IssueSelected state."
            )?;
            state.issue = state::Issue::Selected(Issue {
                key: String::from("123"),
                summary: String::from("Test issue"),
            });
            Ok(RunType::DryRun { state, stdout })
        }
        RunType::Real(state) => {
            let github_config = state
                .github_config
                .as_ref()
                .ok_or(StepError::GitHubNotConfigured)?;
            let (github, issues) = github::list_issues(github_config, state.github, labels)?;
            let issue = select(issues, "Select an Issue")?;
            println!("Selected item : {}", &issue);
            Ok(RunType::Real(State {
                github,
                issue: state::Issue::Selected(issue),
                ..state
            }))
        }
    }
}

pub(super) fn transition_jira_issue(status: &str, run_type: RunType) -> Result<RunType, StepError> {
    let (state, dry_run_stdout) = run_type.decompose();
    let issue = match &state.issue {
        state::Issue::Selected(issue) => issue,
        state::Issue::Initial => return Err(StepError::NoIssueSelected),
    };
    let jira_config = state
        .jira_config
        .as_ref()
        .ok_or(StepError::JiraNotConfigured)?;

    if let Some(mut stdout) = dry_run_stdout {
        writeln!(
            stdout,
            "Would transition currently selected issue to status {status}"
        )?;
        return Ok(RunType::DryRun { state, stdout });
    }

    jira::transition_issue(jira_config, &issue.key, status)?;
    let key = &issue.key;
    println!("{key} transitioned to {status}");
    Ok(RunType::Real(state))
}
