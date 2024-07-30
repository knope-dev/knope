use std::{io::Write, path::PathBuf};

use datta::UriTemplate;
use miette::Diagnostic;

use crate::{
    app_config, config,
    dry_run::DryRun,
    integrations::{
        github::initialize_state, ureq_err_to_string, CreateReleaseInput, CreateReleaseResponse,
    },
    state,
    step::releases::package::{Asset, AssetNameError},
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_release(
    name: &str,
    tag_name: &str,
    body: &str,
    prerelease: bool,
    github_state: state::GitHub,
    github_config: &config::GitHub,
    dry_run_stdout: DryRun,
    assets: Option<&Vec<Asset>>,
) -> Result<state::GitHub, Error> {
    let github_release =
        CreateReleaseInput::new(tag_name, name, body, prerelease, assets.is_some());

    if let Some(stdout) = dry_run_stdout {
        github_release_dry_run(name, assets, &github_release, stdout)?;
        return Ok(github_state);
    }

    let (token, agent) = initialize_state(github_state)?;

    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/releases",
        owner = github_config.owner,
        repo = github_config.repo,
    );
    let token_header = format!("token {}", &token);

    let response: CreateReleaseResponse = agent
        .post(&url)
        .set("Authorization", &token_header)
        .send_json(github_release)
        .map_err(|source| Error::ApiRequest {
            err: ureq_err_to_string(source),
            activity: "creating a release".to_string(),
        })?
        .into_json()
        .map_err(|source| Error::ApiResponse {
            source,
            activity: "creating a release",
        })?;

    if let Some(assets) = assets {
        let mut upload_template = UriTemplate::new(&response.upload_url);
        for asset in assets {
            let file =
                std::fs::read(&asset.path).map_err(|source| Error::CouldNotReadAssetFile {
                    path: asset.path.clone(),
                    source,
                })?;
            let asset_name = asset.name()?;
            let upload_url = upload_template.set("name", asset_name.as_str()).build();
            agent
                .post(&upload_url)
                .set("Authorization", &token_header)
                .set("Content-Type", "application/octet-stream")
                .set("Content-Length", &file.len().to_string())
                .send_bytes(&file)
                .map_err(|source| Error::ApiRequest {
                    err: ureq_err_to_string(source),
                    activity: format!(
                        "uploading asset {asset_name}. Release has been created but not published!",
                    ),
                })?;
        }
        agent
            .patch(&response.url)
            .set("Authorization", &token_header)
            .send_json(ureq::json!({
                "draft": false
            }))
            .map_err(|source| Error::ApiRequest {
                err: ureq_err_to_string(source),
                activity: "publishing release".to_string(),
            })?;
    }

    Ok(state::GitHub::Initialized { token, agent })
}

fn github_release_dry_run(
    name: &str,
    assets: Option<&Vec<Asset>>,
    github_release: &CreateReleaseInput,
    stdout: &mut Box<dyn Write>,
) -> Result<(), Error> {
    let release_type = if github_release.prerelease {
        "prerelease"
    } else {
        "release"
    };
    let body = github_release.body.as_ref().map_or_else(
        || String::from("autogenerated body"),
        |body| format!("body:\n{body}"),
    );
    writeln!(
        stdout,
        "Would create a {release_type} on GitHub with name {name} and tag {tag} and {body}",
        tag = github_release.tag_name
    )
    .map_err(Error::Stdout)?;

    if let Some(assets) = assets {
        writeln!(stdout, "Would upload assets to GitHub:").map_err(Error::Stdout)?;
        for asset in assets {
            let asset_name = asset.name()?;
            writeln!(
                stdout,
                "- {asset_name} from {path}",
                path = asset.path.display(),
            )
            .map_err(Error::Stdout)?;
        }
    }
    Ok(())
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(
        "Could not read asset file {path}: {source} Release has been created but not published!"
    )]
    #[diagnostic(
    code(step::could_not_read_asset_file),
    help("This could be a permissions issue or the file may not exist relative to the current working directory.")
    )]
    CouldNotReadAssetFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(transparent)]
    #[diagnostic(transparent)]
    AppConfig(#[from] app_config::Error),
    #[error("Trouble communicating with GitHub while {activity}: {err}")]
    #[diagnostic(
        code(github::api_request_error),
        help(
            "There was a problem communicating with GitHub, this may be a network issue or a permissions issue."
        )
    )]
    ApiRequest { err: String, activity: String },
    #[error("Trouble decoding the response from GitHub while {activity}: {source}")]
    #[diagnostic(
        code(github::api_response_error),
        help(
            "Failure to decode a response from GitHub is probably a bug. Please report it at https://github.com/knope-dev/knope"
        )
    )]
    ApiResponse {
        source: std::io::Error,
        activity: &'static str,
    },
    #[error("Could not write to stdout")]
    Stdout(std::io::Error),
    #[error("Asset was not uploaded to GitHub, a release was created but is still a draft! {0}")]
    #[diagnostic(
        code(github::asset_name_error),
        help("Try setting the `name` property of the asset manually"),
        url("https://knope.tech/reference/config-file/packages/#assets")
    )]
    AssetName(#[from] AssetNameError),
}
