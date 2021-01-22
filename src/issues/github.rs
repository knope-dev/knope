use color_eyre::eyre::{eyre, Result};

use crate::app_config::get_or_prompt_for_octocrab;
use crate::issues::Issue;
use crate::{config, state};

pub(crate) async fn list_issues(
    github_config: Option<config::GitHub>,
    github_state: state::GitHub,
    labels: Option<Vec<String>>,
) -> Result<(Option<config::GitHub>, state::GitHub, Vec<Issue>)> {
    match github_config {
        None => Err(eyre!("GitHub is not configured")),
        Some(github_config) => {
            let octocrab = match github_state {
                state::GitHub::Initialized { octocrab } => octocrab,
                state::GitHub::New => get_or_prompt_for_octocrab()?,
            };
            let request = octocrab.issues(&github_config.owner, &github_config.repo);
            let oc_issues = if let Some(labels) = labels {
                request.list().labels(labels.as_slice()).send().await?
            } else {
                request.list().send().await?
            };
            let issues = oc_issues
                .items
                .into_iter()
                .filter_map(|oc_issue| match oc_issue.pull_request {
                    Some(_) => None,
                    None => Some(Issue {
                        key: oc_issue.number.to_string(),
                        summary: oc_issue.title,
                    }),
                })
                .collect();

            Ok((
                Some(github_config),
                state::GitHub::Initialized { octocrab },
                issues,
            ))
        }
    }
}
