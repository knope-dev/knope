use ureq::Agent;

use crate::{
    app_config::{self, get_or_prompt_for_gitea_token},
    state,
};

pub(crate) mod create_pull_request;
pub(crate) mod create_release;
mod list_issues;

pub(crate) use create_pull_request::{
    Error as CreatePullRequestError, create_or_update_pull_request,
};
pub(crate) use create_release::{Error as CreateReleaseError, create_release};
pub(crate) use list_issues::{Error as ListIssuesError, list_issues};

fn initialize_state(host: &str, state: state::Gitea) -> Result<(String, Agent), app_config::Error> {
    Ok(match state {
        state::Gitea::Initialized { token, agent } => (token, agent),
        state::Gitea::New => {
            let token = get_or_prompt_for_gitea_token(host)?;
            (token, Agent::new())
        }
    })
}
