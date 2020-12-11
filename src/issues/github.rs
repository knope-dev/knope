use color_eyre::eyre::Result;

use crate::app_config::get_or_prompt_for_octocrab;
use crate::issues::Issue;
use crate::{config, state};

pub(crate) async fn list_issues(
    github_config: Option<config::GitHub>,
    github_state: state::GitHub,
) -> Result<(Option<config::GitHub>, state::GitHub, Vec<Issue>)> {
    match github_config {
        None => Ok((github_config, github_state, vec![])),
        Some(github_config) => {
            let octocrab = match github_state {
                state::GitHub::Initialized { octocrab } => octocrab,
                state::GitHub::New => get_or_prompt_for_octocrab()?,
            };
            let issues: Vec<Issue> = octocrab
                .issues(&github_config.owner, &github_config.repo)
                .list()
                .send()
                .await?
                .items
                .into_iter()
                .map(|oc_issue| Issue::GitHub {
                    number: oc_issue.number,
                    title: oc_issue.title,
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
