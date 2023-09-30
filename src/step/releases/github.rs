use miette::{diagnostic, Diagnostic};

use super::{package::Asset, PackageName, Release, TimeError};
use crate::{config::GitHub, dry_run::DryRun, integrations::github as api, state};

pub(crate) fn release(
    package_name: Option<&PackageName>,
    release: &Release,
    github_state: state::GitHub,
    github_config: &GitHub,
    dry_run_stdout: DryRun,
    assets: Option<&Vec<Asset>>,
    tag: &str,
) -> Result<state::GitHub, Error> {
    let version = &release.new_version;
    let release_title = release.title()?;

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

    api::create_release(
        &name,
        tag,
        body.as_deref(),
        version.is_prerelease(),
        github_state,
        github_config,
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
