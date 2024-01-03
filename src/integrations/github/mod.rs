pub(crate) use create_pull_request::{
    create_or_update_pull_request, Error as CreatePullRequestError,
};
pub(crate) use create_release::{create_release, Error as CreateReleaseError};
use ureq::Agent;

use crate::{app_config, app_config::get_or_prompt_for_github_token, state};

mod create_pull_request;
mod create_release;

fn initialize_state(state: state::GitHub) -> Result<(String, Agent), app_config::Error> {
    Ok(match state {
        state::GitHub::Initialized { token, agent } => (token, agent),
        state::GitHub::New => {
            let token = get_or_prompt_for_github_token()?;
            (token, Agent::new())
        }
    })
}
