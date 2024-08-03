pub(crate) use api::CreateReleaseError as Error;
use knope_versioning::{package, release_notes::Release, ReleaseTag};

use super::package::Asset;
use crate::{config::GitHub, integrations::github as api, state, state::RunType};

pub(crate) fn release(
    package_name: &package::Name,
    release: &Release,
    github_state: RunType<state::GitHub>,
    github_config: &GitHub,
    assets: Option<&Vec<Asset>>,
    tag: &ReleaseTag,
) -> Result<state::GitHub, Error> {
    let version = &release.version;
    let mut name = if let package::Name::Custom(package_name) = package_name {
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
