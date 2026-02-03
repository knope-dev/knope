pub mod changelog_section;
mod package;
mod release_notes;
mod template;

use serde::{Deserialize, Serialize};

pub use self::{
    changelog_section::ChangelogSection,
    package::{Asset, AssetNameError, Assets, Package, VersionedFile},
    template::{Template, Variable},
};
pub use crate::release_notes::ReleaseNotes;

/// Configuration for how changes are tracked and processed
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Changes {
    /// If set to true, conventional commits are ignored across all workflows
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ignore_conventional_commits: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub release_notes: Option<ReleaseNotes>,
    pub changes: Option<Changes>,
}
