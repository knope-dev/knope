use std::io::Write;

use indexmap::IndexMap;

use super::Package;
use crate::{
    config::ChangeLogSectionName,
    releases::{semver::Version, ChangeType, Release},
    step::StepError,
};

impl Package {
    /// Adds content from `release` to `Self::changelog` if it exists.
    pub(crate) fn write_changelog(
        &mut self,
        version: Version,
        dry_run: &mut Option<Box<dyn Write>>,
    ) -> Result<Release, StepError> {
        let mut fixes = Vec::new();
        let mut features = Vec::new();
        let mut breaking_changes = Vec::new();
        let mut extra_sections: IndexMap<ChangeLogSectionName, Vec<String>> = IndexMap::new();

        for change in &self.pending_changes {
            match change.change_type() {
                ChangeType::Fix => fixes.push(change.summary()),
                ChangeType::Feature => features.push(change.summary()),
                ChangeType::Breaking => breaking_changes.push(change.summary()),
                ChangeType::Custom(source) => {
                    if let Some(section) = self.extra_changelog_sections.get(&source) {
                        extra_sections
                            .entry(section.clone())
                            .or_default()
                            .push(change.summary());
                    }
                }
            }
        }

        let new_changelog_body = new_changelog(fixes, features, breaking_changes, extra_sections);
        let release = Release::new(new_changelog_body, version);
        let new_changelog = release.changelog_entry()?;

        if let Some(changelog) = self.changelog.as_mut() {
            changelog.content = add_version_to_changelog(&changelog.content, &new_changelog);
            if let Some(stdout) = dry_run {
                writeln!(
                    stdout,
                    "Would add the following to {}:",
                    changelog.path.display()
                )?;
                writeln!(stdout, "{}", &new_changelog)?;
            } else {
                std::fs::write(&changelog.path, &changelog.content)?;
            }
        };

        Ok(release)
    }
}

/// Take in some existing markdown in the expected changelog format, find the top entry, and
/// put the new version above it.
pub(super) fn add_version_to_changelog(existing: &str, new_changes: &str) -> String {
    let mut changelog = String::new();
    let mut not_written = true;

    for line in existing.lines() {
        if line.starts_with("##") && not_written {
            changelog.push_str(new_changes);
            changelog.push('\n');
            not_written = false;
        }
        changelog.push_str(line);
        changelog.push('\n');
    }

    if not_written {
        changelog.push_str(new_changes);
    }

    if existing.ends_with('\n') && !changelog.ends_with('\n') {
        // Preserve white space at end of file
        changelog.push('\n');
    }

    changelog
}

pub(super) fn new_changelog(
    fixes: Vec<String>,
    features: Vec<String>,
    breaking_changes: Vec<String>,
    extra_sections: IndexMap<ChangeLogSectionName, Vec<String>>,
) -> String {
    let mut blocks = Vec::new();

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
    blocks.join("\n")
}

fn create_section(title: &str, items: Vec<String>) -> Vec<String> {
    let mut blocks = Vec::with_capacity(items.len() + 2);
    blocks.push(format!("### {title}"));
    blocks.extend(items.into_iter().map(|summary| {
        if summary.starts_with("#### ") {
            // Sometimes the formatting is already done, like in changesets
            format!("\n{summary}")
        } else {
            format!("\n#### {summary}")
        }
    }));
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
        let new_changes = new_changelog(
            vec!["Fixed something".to_string()],
            vec![String::from("New Feature"), String::from("Another feature")],
            vec![String::from("Breaking change")],
            extra_sections,
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

### Breaking Changes

#### Breaking change

### Features

#### New Feature

### Fixes

#### Fixed something
"##;

        let new_changes = new_changelog(
            vec!["Fixed something".to_string()],
            vec![String::from("New Feature")],
            vec![String::from("Breaking change")],
            IndexMap::new(),
        );
        let changelog = add_version_to_changelog(MARKDOWN, &new_changes);
        assert_eq!(changelog, EXPECTED);
    }
}
