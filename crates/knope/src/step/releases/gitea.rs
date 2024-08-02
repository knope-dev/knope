use knope_versioning::{
    package,
    release_notes::{release_title, TimeError},
    CreateRelease, ReleaseTag,
};
use miette::{diagnostic, Diagnostic};

use crate::{config, integrations::gitea as api, state, state::RunType};

pub(crate) fn release(
    package_name: &package::Name,
    release: &CreateRelease,
    gitea_state: RunType<state::Gitea>,
    gitea_config: &config::Gitea,
    tag: &ReleaseTag,
) -> Result<state::Gitea, Error> {
    let version = &release.version;
    let mut name = if let package::Name::Custom(package_name) = package_name {
        format!("{package_name} ")
    } else {
        String::new()
    };
    name.push_str(&release_title(version, None, true)?);

    api::create_release(
        &name,
        tag.as_str(),
        release.notes.trim(),
        version.is_prerelease(),
        gitea_state,
        gitea_config,
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
