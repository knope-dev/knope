pub(crate) use api::CreateReleaseError as Error;
use knope_versioning::{ReleaseTag, package, release_notes::Release};

use crate::{config, integrations::gitea as api, state, state::RunType};

pub(crate) fn release(
    release: &Release,
    gitea_state: RunType<state::Gitea>,
    gitea_config: &config::Gitea,
    tag: &ReleaseTag,
) -> Result<state::Gitea, Error> {
    let version = &release.version;
    let mut name = if let package::Name::Custom(package_name) = &release.package_name {
        format!("{package_name} ")
    } else {
        String::new()
    };
    name.push_str(&release.title);

    api::create_release(
        &name,
        tag.as_str(),
        release.notes.trim(),
        version.is_prerelease(),
        gitea_state,
        gitea_config,
    )
}
