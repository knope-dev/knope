use std::iter::Map;
use std::slice::Iter;

use itertools::Itertools;

/// Take in some existing markdown in the expected changelog format, find the top entry, and
/// put the new version above it.
pub(crate) fn add_version_to_changelog(existing: &str, version: &Version) -> String {
    let new_changes = version.markdown_lines();

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

#[derive(Clone)]
pub(crate) struct Version {
    pub(crate) title: String,
    pub(crate) fixes: Vec<String>,
    pub(crate) features: Vec<String>,
    pub(crate) breaking_changes: Vec<String>,
}

impl Version {
    fn markdown_lines(&self) -> Vec<String> {
        const HEADERS_AND_PADDING: usize = 10;
        let Self {
            title,
            fixes,
            features,
            breaking_changes,
        } = self;
        let mut blocks = Vec::with_capacity(
            fixes.len() + features.len() + breaking_changes.len() + HEADERS_AND_PADDING,
        );

        blocks.push(format!("## {}\n", title));
        if !breaking_changes.is_empty() {
            blocks.push(String::from("### Breaking Changes\n"));
            blocks.extend(unordered_list(breaking_changes));
            blocks.push(String::new());
        }
        if !features.is_empty() {
            blocks.push(String::from("### Features\n"));
            blocks.extend(unordered_list(features));
            blocks.push(String::new());
        }
        if !fixes.is_empty() {
            blocks.push(String::from("### Fixes\n"));
            blocks.extend(unordered_list(fixes));
            blocks.push(String::new());
        }
        blocks
    }
}

fn unordered_list(items: &[String]) -> Map<Iter<String>, fn(&String) -> String> {
    items.iter().map(|note| format!("- {}", note))
}

#[cfg(test)]
mod tests {
    use crate::changelog::add_version_to_changelog;

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

- Breaking change

### Features

- New Feature

### Fixes

- Fixed something

## 0.1.0 - 2020-12-25
### Features
- Initial version

[link]: some footer details
"##;

        let version = super::Version {
            title: "0.2.0 - 2020-12-31".to_string(),
            fixes: vec!["Fixed something".to_string()],
            features: vec![String::from("New Feature")],
            breaking_changes: vec![String::from("Breaking change")],
        };
        let changelog = add_version_to_changelog(MARKDOWN, &version);
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

- Breaking change

### Features

- New Feature

### Fixes

- Fixed something
"##;

        let version = super::Version {
            title: "0.2.0 - 2020-12-31".to_string(),
            fixes: vec!["Fixed something".to_string()],
            features: vec![String::from("New Feature")],
            breaking_changes: vec![String::from("Breaking change")],
        };
        let changelog = add_version_to_changelog(MARKDOWN, &version);
        assert_eq!(changelog, EXPECTED);
    }
}
