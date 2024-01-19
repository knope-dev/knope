use miette::Diagnostic;

use crate::{
    integrations::{gitea, github},
    state::RunType,
    variables,
    variables::{replace_variables, Template},
    workflow::Verbose,
};

pub(super) async fn run(
    base: &str,
    title: Template,
    body: Template,
    run_type: RunType,
) -> Result<RunType, Error> {
    let (mut state, mut dry_run) = run_type.decompose();
    let title = replace_variables(title, &state)?;
    let body = replace_variables(body, &state)?;

    if state.github_config.is_none() && state.gitea_config.is_none() {
        return Err(Error::NotConfigured);
    }
    let client = state.get_client();

    if let Some(github_config) = &state.github_config {
        state.github = github::create_or_update_pull_request(
            &title,
            &body,
            base,
            state.github,
            github_config,
            &mut dry_run,
            Verbose::Yes,
            client.clone(),
        )
        .await?;
    }

    // TODO: Do this in parallel with GitHub
    if let Some(gitea_config) = &state.gitea_config {
        state.gitea = gitea::create_or_update_pull_request(
            &title,
            &body,
            base,
            state.gitea,
            gitea_config,
            &mut dry_run,
            Verbose::Yes,
            client.clone(),
        )
        .await?;
    }
    Ok(RunType::recompose(state, dry_run))
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
