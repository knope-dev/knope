use std::{io::Write, path::PathBuf};

use datta::UriTemplate;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};

use crate::{
    app_config,
    app_config::get_or_prompt_for_github_token,
    config::GitHub,
    releases,
    releases::{git::tag_name, package::Asset, PackageName, Release},
    state,
    state::GitHub::{Initialized, New},
};

pub(crate) fn release(
    package_name: Option<&PackageName>,
    release: &Release,
    github_state: state::GitHub,
    github_config: &GitHub,
    dry_run_stdout: Option<&mut Box<dyn Write>>,
    assets: Option<&Vec<Asset>>,
) -> Result<state::GitHub, Error> {
    let version = &release.new_version;
    let release_title = release.title()?;

    let tag_name = tag_name(version, package_name);
    let name = if let Some(package_name) = package_name {
        format!("{package_name} {release_title}")
    } else {
        release_title
    };

    let body = release.new_changelog.as_ref().map(|new_changelog| {
        new_changelog
            .lines()
            .map(|line| {
                if line.starts_with("##") {
                    #[allow(clippy::indexing_slicing)] // Just checked len above
                    &line[1..] // Reduce header level by one
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    });

    let github_release = GitHubRelease::new(
        &tag_name,
        &name,
        body,
        version.is_prerelease(),
        assets.is_some(),
    );

    if let Some(stdout) = dry_run_stdout {
        github_release_dry_run(&name, assets, &github_release, stdout)?;
        return Ok(github_state);
    }

    let token = match github_state {
        Initialized { token } => token,
        New => get_or_prompt_for_github_token()?,
    };

    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/releases",
        owner = github_config.owner,
        repo = github_config.repo,
    );
    let token_header = format!("token {}", &token);

    let response: CreateReleaseResponse = ureq::post(&url)
        .set("Authorization", &token_header)
        .send_json(github_release)
        .map_err(|source| Error::ApiRequest {
            source: Box::new(source),
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
            let file = std::fs::File::open(&asset.path).map_err(|source| {
                Error::CouldNotReadAssetFile {
                    path: asset.path.clone(),
                    source,
                }
            })?;
            let upload_url = upload_template.set("name", asset.name.as_str()).build();
            ureq::post(&upload_url)
                .set("Authorization", &token_header)
                .send(file)
                .map_err(|source| Error::ApiRequest {
                    source: Box::new(source),
                    activity: format!(
                        "uploading asset {name}. Release has been created but not published!"
                    ),
                })?;
        }
        ureq::patch(&response.url)
            .set("Authorization", &token_header)
            .send_json(ureq::json!({
                "draft": false
            }))
            .map_err(|source| Error::ApiRequest {
                source: Box::new(source),
                activity: "publishing release".to_string(),
            })?;
    }

    Ok(Initialized { token })
}

fn github_release_dry_run(
    name: &str,
    assets: Option<&Vec<Asset>>,
    github_release: &GitHubRelease,
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
            writeln!(
                stdout,
                "- {name} from {path}",
                name = asset.name,
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
    AppConfig(#[from] app_config::Error),
    #[error("Trouble communicating with GitHub while {activity}: {source}")]
    #[diagnostic(
        code(github::api_request_error),
        help(
            "There was a problem communicating with GitHub, this may be a network issue or a permissions issue."
        )
    )]
    ApiRequest {
        source: Box<ureq::Error>,
        activity: String,
    },
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
    #[error(transparent)]
    Release(#[from] releases::Error),
}

#[derive(Serialize)]
struct GitHubRelease<'a> {
    tag_name: &'a str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
    prerelease: bool,
    /// Whether to automatically generate the body for this release.
    /// If body is specified, the body will be pre-pended to the automatically generated notes.
    generate_release_notes: bool,
    /// true to create a draft (unpublished) release, false to create a published one.
    draft: bool,
}

impl<'a> GitHubRelease<'a> {
    fn new(
        tag_name: &'a str,
        name: &'a str,
        body: Option<String>,
        prerelease: bool,
        draft: bool,
    ) -> Self {
        Self {
            generate_release_notes: body.is_none(),
            tag_name,
            name,
            body,
            prerelease,
            draft,
        }
    }
}

#[derive(Deserialize)]
struct CreateReleaseResponse {
    url: String,
    upload_url: String,
}
