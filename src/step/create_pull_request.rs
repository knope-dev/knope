use miette::Diagnostic;

use crate::{
    integrations::github::{create_or_update_pull_request, CreatePullRequestError},
    state::RunType,
    variables,
    variables::{replace_variables, Template},
    workflow::Verbose,
};

pub(super) fn run(
    base: &str,
    title: Template,
    body: Template,
    run_type: RunType,
) -> Result<RunType, Error> {
    let (mut state, mut dry_run) = run_type.decompose();
    let title = replace_variables(title, &state)?;
    let body = replace_variables(body, &state)?;
    let Some(github_config) = &state.github_config else {
        return Err(Error::NotConfigured);
    };
    state.github = create_or_update_pull_request(
        &title,
        &body,
        base,
        state.github,
        github_config,
        &mut dry_run,
        Verbose::Yes,
    )?;
    Ok(RunType::recompose(state, dry_run))
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Variables(#[from] variables::Error),
    #[error("GitHub is not configured")]
    #[diagnostic(
        code(create_pull_request::github::not_configured),
        help("GitHub must be configured in order to use the CreatePullRequest step"),
        url("https://knope-dev.github.io/knope/config/github.html")
    )]
    NotConfigured,
    #[error(transparent)]
    #[diagnostic(transparent)]
    GitHub(#[from] CreatePullRequestError),
}
