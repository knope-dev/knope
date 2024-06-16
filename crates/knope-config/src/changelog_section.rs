use knope_versioning::{
    changelog::{CommitFooter, CustomChangeType, SectionName, Sections},
    changes::ChangeType,
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

// pub fn convert_from_versioning(sections: Sections) -> Vec<ChangelogSection> {
//     let defaults = Sections::defaults();
//     let mut sections = sections.0;
//     sections.retain(|(name, source)| {
//         source.len() != 1
//             || !defaults.iter().any(|(default_name, default_source)| {
//                 default_name == name && source.first().is_some_and(|it| it == default_source)
//             })
//     });
//     sections
//         .into_iter()
//         .map(|(name, sources)| ChangelogSection {
//             name,
//             footers: sources
//                 .iter()
//                 .filter_map(|source| match source {
//                     ChangeType::Custom(SectionSource::CommitFooter(footer)) => Some(footer.clone()),
//                     _ => None,
//                 })
//                 .collect(),
//             types: sources
//                 .iter()
//                 .filter_map(|source| match source {
//                     ChangeType::Custom(SectionSource::CustomChangeType(change_type)) => {
//                         Some(change_type.clone())
//                     }
//                     ChangeType::Breaking => Some("Breaking Changes".into()),
//                     ChangeType::Feature => Some("Features".into()),
//                     ChangeType::Fix => Some("Fixes".into()),
//                     ChangeType::Custom(SectionSource::CommitFooter(_)) => None,
//                 })
//                 .collect(),
//         })
//         .collect()
// }
