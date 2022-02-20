pub(crate) use conventional_commits::update_project_from_conventional_commits as prepare_release;
pub(crate) use github::release;

pub(crate) use self::semver::bump_version_and_update_state as bump_version;
pub(crate) use self::semver::{get_version, Rule};

mod changelog;
mod conventional_commits;
mod github;
mod semver;

pub(crate) struct Release {
    pub(crate) version: ::semver::Version,
    pub(crate) changelog: String,
}
