use miette::{diagnostic, Diagnostic};

use super::{package::Asset, PackageName, Release, TimeError};
use crate::{config, dry_run::DryRun, integrations::gitea as api, state};

pub(crate) fn release(
    package_name: Option<&PackageName>,
    release: &Release,
    gitea_state: state::Gitea,
    gitea_config: &config::Gitea,
    dry_run_stdout: DryRun,
    assets: Option<&Vec<Asset>>,
    tag: &str,
) -> Result<state::Gitea, Error> {
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
        gitea_state,
        gitea_config,
        dry_run_stdout,
        assets,
    )
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
