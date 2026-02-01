pub mod changelog_section;
mod package;
mod release_notes;
mod template;

use serde::Deserialize;

pub use self::{
    changelog_section::ChangelogSection,
    package::{Asset, AssetNameError, Assets, Package, VersionedFile},
    template::{Template, Variable},
};
pub use crate::release_notes::ReleaseNotes;

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub release_notes: Option<ReleaseNotes>,
    /// If set to true, conventional commits are ignored across all workflows
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ignore_conventional_commits: bool,
}
