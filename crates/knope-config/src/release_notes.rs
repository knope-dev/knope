use knope_versioning::release_notes::ChangeTemplate;
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ReleaseNotes {
    pub change_templates: Vec<ChangeTemplate>,
}
