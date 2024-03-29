use std::io::Write;

use miette::Diagnostic;

use super::initialize_state;
use crate::{
    app_config, config,
    dry_run::DryRun,
    integrations::{ureq_err_to_string, CreateReleaseInput, CreateReleaseResponse},
    state,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_release(
    name: &str,
    tag_name: &str,
    body: Option<&str>,
    prerelease: bool,
    gitea_state: state::Gitea,
    gitea_config: &config::Gitea,
    dry_run_stdout: DryRun,
) -> Result<state::Gitea, Error> {
    let gitea_release = CreateReleaseInput::new(tag_name, name, body, prerelease, false);

    if let Some(stdout) = dry_run_stdout {
        gitea_release_dry_run(name, gitea_config, &gitea_release, stdout)?;
        return Ok(gitea_state);
    }

    let (token, agent) = initialize_state(&gitea_config.host, gitea_state)?;

    agent
        .post(&gitea_config.get_releases_url())
        .query("access_token", &token)
        .send_json(gitea_release)
        .map_err(|source| Error::ApiRequest {
            err: ureq_err_to_string(source),
            activity: "creating a release".to_string(),
            host: gitea_config.host.clone(),
        })?
        .into_json::<CreateReleaseResponse>()
        .map_err(|source| Error::ApiResponse {
            source,
            activity: "creating a release",
            host: gitea_config.host.clone(),
        })?;

    Ok(state::Gitea::Initialized { token, agent })
}

fn gitea_release_dry_run(
    name: &str,
    config: &config::Gitea,
    gitea_release: &CreateReleaseInput,
    stdout: &mut Box<dyn Write>,
) -> Result<(), Error> {
    let release_type = if gitea_release.prerelease {
        "prerelease"
    } else {
        "release"
    };
    let body = gitea_release.body.as_ref().map_or_else(
        || String::from("autogenerated body"),
        |body| format!("body:\n{body}"),
    );
    writeln!(
        stdout,
        "Would create a {release_type} on Gitea [{host}] with name {name} and tag {tag} and {body}",
        tag = gitea_release.tag_name,
        host = config.host
    )
    .map_err(Error::Stdout)?;

    Ok(())
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    AppConfig(#[from] app_config::Error),
    #[error("Trouble communicating with the Gitea instance while {activity}: {err}")]
    #[diagnostic(
        code(gitea::api_request_error),
        help(
            "There was a problem communicating with the Gitea instance {host}, this may be a network issue or a permissions issue."
        )
    )]
    ApiRequest {
        err: String,
        activity: String,
        host: String,
    },
    #[error("Trouble decoding the response from Gitea while {activity}: {source}")]
    #[diagnostic(
        code(gitea::api_response_error),
        help(
            "Failure to decode a response from the Gitea instance at {host} is probably a bug. Please report it at https://github.com/knope-dev/knope"
        )
    )]
    ApiResponse {
        source: std::io::Error,
        activity: &'static str,
        host: String,
    },
    #[error("Could not write to stdout")]
    Stdout(std::io::Error),
}
