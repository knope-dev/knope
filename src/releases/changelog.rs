use indexmap::IndexMap;
use itertools::Itertools;

use crate::config::ChangeLogSectionName;

/// Take in some existing markdown in the expected changelog format, find the top entry, and
/// put the new version above it.
pub(super) fn add_version_to_changelog(existing: &str, new_changes: &[String]) -> String {
    let mut lines = existing.lines();
    let mut changelog = lines
        .take_while_ref(|line| !line.starts_with("##"))
        .chain(new_changes.iter().map(String::as_str))
        .join("\n");

    if let Some(existing) = lines.next() {
        // Give an extra space between the new section and the existing section.
        changelog.push('\n');
        changelog.push_str(existing);
        changelog.push('\n');
    }
    changelog.push_str(&lines.join("\n"));

    if existing.ends_with('\n') && !changelog.ends_with('\n') {
        // Preserve whitespace at end of the file.
        changelog.push('\n');
    }
    changelog
}

pub(super) fn new_changelog_lines(
    title: &str,
    fixes: &[String],
    features: &[String],
    breaking_changes: &[String],
    extra_sections: &IndexMap<ChangeLogSectionName, Vec<String>>,
) -> Vec<String> {
    const HEADERS_AND_PADDING: usize = 10;
    let mut blocks = Vec::with_capacity(
        fixes.len() + features.len() + breaking_changes.len() + HEADERS_AND_PADDING,
    );

    blocks.push(format!("## {title}\n"));
    if !breaking_changes.is_empty() {
        blocks.extend(create_section("Breaking Changes", breaking_changes));
    }
    if !features.is_empty() {
        blocks.extend(create_section("Features", features));
    }
    if !fixes.is_empty() {
        blocks.extend(create_section("Fixes", fixes));
    }
    for (section_title, notes) in extra_sections {
        blocks.extend(create_section(section_title.as_ref(), notes));
    }
    blocks
}

fn create_section(title: &str, items: &[String]) -> Vec<String> {
    let mut blocks = Vec::with_capacity(items.len() + 2);
    blocks.push(format!("### {title}"));
    blocks.extend(items.iter().map(|summary| format!("\n#### {summary}")));
    blocks.push(String::new());
    blocks
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn changelog_add_version() {
        const MARKDOWN: &str = r##"# Changelog

Some details about the keepachangelog format

Sometimes a second paragraph

## 0.1.0 - 2020-12-25
### Features
- Initial version

[link]: some footer details
"##;
        const EXPECTED: &str = r##"# Changelog

Some details about the keepachangelog format

Sometimes a second paragraph

## 0.2.0 - 2020-12-31

### Breaking Changes

#### Breaking change

### Features

#### New Feature

#### Another feature

### Fixes

#### Fixed something

### Notes

#### Something

### More stuff

#### stuff

## 0.1.0 - 2020-12-25
### Features
- Initial version

[link]: some footer details
"##;

        let mut extra_sections = IndexMap::new();
        extra_sections.insert(
            ChangeLogSectionName::from("Notes"),
            vec![String::from("Something")],
        );
        extra_sections.insert(
            ChangeLogSectionName::from("More stuff"),
            vec![String::from("stuff")],
        );
        let new_changes = new_changelog_lines(
            "0.2.0 - 2020-12-31",
            &["Fixed something".to_string()],
            &[String::from("New Feature"), String::from("Another feature")],
            &[String::from("Breaking change")],
            &extra_sections,
        );
        let changelog = add_version_to_changelog(MARKDOWN, &new_changes);
        assert_eq!(changelog, EXPECTED);
    }

    #[test]
    fn changelog_no_existing_version() {
        const MARKDOWN: &str = r##"# Changelog

Some details about the keepachangelog format

Sometimes a second paragraph

"##;
        const EXPECTED: &str = r##"# Changelog

Some details about the keepachangelog format

Sometimes a second paragraph

## 0.2.0 - 2020-12-31

### Breaking Changes

#### Breaking change

### Features

#### New Feature

### Fixes

#### Fixed something
"##;

        let new_changes = new_changelog_lines(
            "0.2.0 - 2020-12-31",
            &["Fixed something".to_string()],
            &[String::from("New Feature")],
            &[String::from("Breaking change")],
            &IndexMap::new(),
        );
        let changelog = add_version_to_changelog(MARKDOWN, &new_changes);
        assert_eq!(changelog, EXPECTED);
    }
}
