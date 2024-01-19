use super::Issue;
pub(crate) use crate::integrations::gitea::ListIssuesError as Error;
use crate::{
    integrations::gitea::{list_issues, ListIssuesError},
    prompt,
    state::{self, RunType, State},
};

pub(crate) async fn select_issue(
    labels: Option<&[String]>,
    run_type: RunType,
) -> Result<RunType, ListIssuesError> {
    match run_type {
        RunType::DryRun { state, .. } if state.gitea_config.is_none() => {
            Err(ListIssuesError::NotConfigured)
        }
        RunType::DryRun {
            mut state,
            mut stdout,
        } => {
            if let Some(labels) = labels {
                writeln!(
                    stdout,
                    "Would query configured Gitea instance for issues with labels {}",
                    labels.join(", ")
                )?;
            } else {
                writeln!(
                    stdout,
                    "Would query configured Gitea instance for issues with any labels"
                )?;
            }

            writeln!(stdout, "Would prompt user to select an issue")?;

            state.issue = state::Issue::Selected(Issue {
                key: String::from("123"),
                summary: String::from("Test issue"),
            });

            Ok(RunType::DryRun { state, stdout })
        }

        RunType::Real(mut state) => {
            let config = state.gitea_config;
            let (gitea, issues) =
                list_issues(&config, state.gitea, labels, state.get_client()).await?;
            let issue = prompt::select(issues, "Select an Issue")?;
            println!("Selected item: {issue}");
            Ok(RunType::Real(State {
                gitea,
                gitea_config: config,
                issue: state::Issue::Selected(issue),
                ..state
            }))
        }
    }
}
