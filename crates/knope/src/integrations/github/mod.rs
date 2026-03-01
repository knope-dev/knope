pub(crate) use create_pull_request::{
    Error as CreatePullRequestError, create_or_update_pull_request,
};
pub(crate) use create_release::{Error as CreateReleaseError, create_release};
pub(crate) use pr_info::enrich_git_info;
use ureq::Agent;

use crate::{app_config, app_config::get_or_prompt_for_github_token, state};

mod create_pull_request;
mod create_release;
mod pr_info;

fn initialize_state(state: state::GitHub) -> Result<(String, Agent), app_config::Error> {
    Ok(match state {
        state::GitHub::Initialized { token, agent } => (token, agent),
        state::GitHub::New => {
            let token = get_or_prompt_for_github_token()?;
            (
                token,
                Agent::new_with_config(
                    Agent::config_builder()
                        .http_status_as_error(false)
                        .accept("application/vnd.github+json")
                        .user_agent("Knope")
                        .build(),
                ),
            )
        }
    })
}
