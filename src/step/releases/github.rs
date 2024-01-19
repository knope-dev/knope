use miette::{diagnostic, Diagnostic};
use reqwest::Client;

use super::{package::Asset, PackageName, Release, TimeError};
use crate::{config::GitHub, dry_run::DryRun, integrations::github as api, state};

pub(crate) async fn release(
    package_name: Option<&PackageName>,
    release: &Release,
    github_state: state::GitHub,
    github_config: &GitHub,
    dry_run_stdout: DryRun,
    assets: Option<&Vec<Asset>>,
    tag: &str,
    client: Client,
) -> Result<state::GitHub, Error> {
    let version = &release.version;
    let mut name = if let Some(package_name) = package_name {
        format!("{package_name} ")
    } else {
        String::new()
    };
    name.push_str(&release.title(false, true)?);

    let body = release.body_at_h1().map(|body| body.trim().to_string());

    api::create_release(
        &name,
        tag,
        body.as_deref(),
        version.is_prerelease(),
        github_state,
        github_config,
        dry_run_stdout,
        assets,
        client,
    )
    .await
    .map_err(Error::from)
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Api(#[from] api::CreateReleaseError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    TimeError(#[from] TimeError),
}
