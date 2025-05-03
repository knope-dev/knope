use knope_config::Template;
use miette::Diagnostic;

use crate::{
    integrations::{gitea, github},
    state::{RunType, State},
    variables,
    variables::replace_variables,
};

pub(super) fn run(
    base: &str,
    title: &Template,
    body: &Template,
    state: RunType<State>,
) -> Result<RunType<State>, Error> {
    let (run_type, mut state) = state.take();
    let title = replace_variables(title, &mut state)?;
    let body = replace_variables(body, &mut state)?;

    if state.github_config.is_none() && state.gitea_config.is_none() {
        return Err(Error::NotConfigured);
    }

    if let Some(github_config) = &state.github_config {
        state.github = github::create_or_update_pull_request(
            &title,
            &body,
            base,
            run_type.of(state.github),
            github_config,
        )?;
    }

    if let Some(gitea_config) = &state.gitea_config {
        state.gitea = gitea::create_or_update_pull_request(
            &title,
            &body,
            base,
            run_type.of(state.gitea),
            gitea_config,
        )?;
    }
    Ok(run_type.of(state))
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Variables(#[from] variables::Error),
    #[error("No forge has been configured")]
    #[diagnostic(
        code(create_pull_request::forge::not_configured),
        help("A forge must be configured in order to use the CreatePullRequest step"),
        url("https://knope.tech/reference/concepts/forge/")
    )]
    NotConfigured,
    #[error(transparent)]
    #[diagnostic(transparent)]
    GitHub(#[from] github::CreatePullRequestError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Gitea(#[from] gitea::CreatePullRequestError),
}
