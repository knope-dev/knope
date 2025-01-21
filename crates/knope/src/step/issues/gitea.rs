use tracing::info;

use super::Issue;
pub(crate) use crate::integrations::gitea::ListIssuesError as Error;
use crate::{
    integrations::gitea::{list_issues, ListIssuesError},
    prompt,
    state::{self, RunType, State},
};

pub(crate) fn select_issue(
    labels: Option<&[String]>,
    state: RunType<State>,
) -> Result<RunType<State>, ListIssuesError> {
    match state {
        RunType::DryRun(state) if state.gitea_config.is_none() => {
            Err(ListIssuesError::NotConfigured)
        }
        RunType::DryRun(mut state) => {
            if let Some(labels) = labels {
                info!(
                    "Would query configured Gitea instance for issues with labels {}",
                    labels.join(", ")
                );
            } else {
                info!("Would query configured Gitea instance for issues with any labels");
            }

            info!("Would prompt user to select an issue");

            state.issue = state::Issue::Selected(Issue {
                key: String::from("123"),
                summary: String::from("Test issue"),
            });

            Ok(RunType::DryRun(state))
        }

        RunType::Real(state) => {
            let config = state.gitea_config;
            let (gitea, issues) = list_issues(config.as_ref(), state.gitea, labels)?;
            let issue = prompt::select(issues, "Select an Issue")?;
            info!("Selected item: {issue}");
            Ok(RunType::Real(State {
                gitea,
                gitea_config: config,
                issue: state::Issue::Selected(issue),
                ..state
            }))
        }
    }
}
