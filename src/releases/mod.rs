pub(crate) use conventional_commits::update_project_from_conventional_commits as prepare_release;
pub(crate) use github::release;

pub(crate) use self::package::{find_packages, suggested_package_toml, Package};
pub(crate) use self::semver::bump_version_and_update_state as bump_version;
pub(crate) use self::semver::{get_version, Rule};

mod cargo;
mod changelog;
mod conventional_commits;
mod github;
mod package;
mod package_json;
mod pyproject;
mod semver;

#[derive(Clone, Debug)]
pub(crate) struct Release {
    pub(crate) version: ::semver::Version,
    pub(crate) changelog: String,
}
