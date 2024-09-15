pub(crate) use api::CreateReleaseError as Error;
use knope_config::Assets;
use knope_versioning::{release_notes::Release, ReleaseTag};

use crate::{config::GitHub, integrations::github as api, state, state::RunType};

pub(crate) fn release(
    release: &Release,
    github_state: RunType<state::GitHub>,
    github_config: &GitHub,
    assets: Option<&Assets>,
    tag: &ReleaseTag,
) -> Result<state::GitHub, Error> {
    let version = &release.version;
    let mut name = if let Some(package_name) = release.package_name.as_custom() {
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
        github_state,
        github_config,
        assets,
    )
}
