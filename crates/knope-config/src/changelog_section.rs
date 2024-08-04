use knope_versioning::{
    changes::ChangeType,
    release_notes::{CommitFooter, CustomChangeType, SectionName, Sections},
};
use serde::{Deserialize, Serialize};

/// <https://knope.tech/reference/config-file/packages/#extra_changelog_sections/>
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChangelogSection {
    pub(crate) name: SectionName,
    #[serde(default)]
    pub(crate) footers: Vec<CommitFooter>,
    #[serde(default)]
    pub(crate) types: Vec<CustomChangeType>,
}

pub fn convert_to_versioning(changelog_sections: Vec<ChangelogSection>) -> Sections {
    let mut defaults = Sections::defaults();
    let mut sections = Vec::with_capacity(changelog_sections.len());
    for ChangelogSection {
        name,
        footers,
        types,
    } in changelog_sections
    {
        let mut sources: Vec<ChangeType> = footers
            .into_iter()
            .map(ChangeType::from)
            .chain(types.into_iter().map(ChangeType::from))
            .collect();
        defaults.retain(|(_, source)| !sources.contains(source));

        // If there's a duplicate section name, combine it
        while let Some((index, (_, change_type))) = defaults
            .iter()
            .enumerate()
            .find(|(_, (default_name, _))| default_name == &name)
        {
            sources.push(change_type.clone());
            defaults.remove(index);
        }
        sections.push((name, sources));
    }
    let defaults = defaults
        .into_iter()
        .map(|(name, source)| (name, vec![source]));
    Sections(defaults.into_iter().chain(sections).collect())
}
