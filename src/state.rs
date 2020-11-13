use color_eyre::eyre::{eyre, Result};

use crate::prompt::select;
use crate::workflow::{JiraConfig, Project};

pub struct Initial {
    pub jira_config: JiraConfig,
    pub projects: Vec<Project>,
}

pub struct Issue {
    pub key: String,
    pub summary: String,
}

pub struct IssueSelected {
    pub jira_config: JiraConfig,
    pub issue: Issue,
    pub projects: Vec<Project>,
}

pub struct ProjectSelected {
    pub jira_config: JiraConfig,
    pub issue: Issue,
    pub project: Project,
}

pub enum State {
    Initial(Initial),
    IssueSelected(IssueSelected),
    ProjectSelected(ProjectSelected),
}

pub fn select_project(state: State) -> Result<ProjectSelected> {
    match state {
        State::Initial { .. } => Err(eyre!("You must select an issue first.")),
        State::ProjectSelected(data) => Ok(data),
        State::IssueSelected(IssueSelected {
            jira_config,
            issue,
            projects,
        }) => {
            let mut candidates: Vec<Project> = projects
                .into_iter()
                .filter(|project| issue.key.starts_with(&project.jira_key))
                .collect();
            if candidates.is_empty() {
                Err(eyre!(
                    "You haven't configured any projects that match the issue key {}",
                    issue.key
                ))
            } else if candidates.len() == 1 {
                Ok(ProjectSelected {
                    jira_config,
                    issue,
                    project: candidates.pop().unwrap(),
                })
            } else {
                let project = select(candidates, "Which project would you like to use?")?;
                Ok(ProjectSelected {
                    jira_config,
                    issue,
                    project,
                })
            }
        }
    }
}
