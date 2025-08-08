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
}
