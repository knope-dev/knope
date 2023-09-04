use std::{fs::read_to_string, io::Write, path::PathBuf};

use indexmap::IndexMap;
use itertools::Itertools;
use miette::Diagnostic;
use thiserror::Error;

use super::Package;
use crate::{
    config::ChangeLogSectionName,
    releases::{semver::Version, ChangeType, Release},
    step::StepError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Changelog {
    pub(crate) path: PathBuf,
    pub(crate) content: String,
}

impl TryFrom<PathBuf> for Changelog {
    type Error = Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let content = if path.exists() {
            read_to_string(&path).map_err(|e| Error::File(path.clone(), e))?
        } else {
            String::new()
        };
        Ok(Self { path, content })
    }
}

impl Changelog {
    pub(super) fn get_section(&self, version: &Version) -> Option<String> {
        let expected_header_start = format!("## {version}");
        let section = self
            .content
            .lines()
            .skip_while(|line| !line.starts_with(&expected_header_start))
            .skip(1) // Skip the header
            .take_while(
                |line| !line.starts_with("## "), // Next version
            )
            .join("\n");
        if section.is_empty() {
            None
        } else {
            Some(section.trim().to_string())
        }
    }
}

#[cfg(test)]
mod test_get_section {
    use std::path::PathBuf;

    use pretty_assertions::assert_eq;

    use crate::releases::{changelog::Changelog, semver::Version};

    const CONTENT: &str = r#"
# Changelog

Hey ya'll this is a changelog

## 0.1.2 2023-05-02

### Features
#### Blah

## 0.1.1 - 2023-03-02

### Fixes

#### it's fixex!

## 0.0.1
Initial release
"#;

    #[test]
    fn first_section() {
        let changelog = Changelog {
            path: PathBuf::default(),
            content: CONTENT.to_string(),
        };

        let section = changelog.get_section(&Version::new(0, 1, 2, None)).unwrap();
        let expected = "### Features\n#### Blah";
        assert_eq!(section, expected);
    }

    #[test]
    fn middle_section() {
        let changelog = Changelog {
            path: PathBuf::default(),
            content: CONTENT.to_string(),
        };

        let section = changelog.get_section(&Version::new(0, 1, 1, None)).unwrap();
        let expected = "### Fixes\n\n#### it's fixex!";
        assert_eq!(section, expected);
    }

    #[test]
    fn no_section() {
        let changelog = Changelog {
            path: PathBuf::default(),
            content: CONTENT.to_string(),
        };

        let section = changelog.get_section(&Version::new(0, 1, 0, None));
        assert!(section.is_none());
    }
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error("Error reading file {0}: {1}")]
    #[diagnostic(
        code(changelog::io),
        help("Please check that the file exists and is readable.")
    )]
    File(PathBuf, #[source] std::io::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

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
        let release = Release::new(Some(new_changelog_body), version);
        let new_changelog = release.changelog_entry()?;

        if let (Some(changelog), Some(new_changes)) = (self.changelog.as_mut(), new_changelog) {
            changelog.content = add_version_to_changelog(&changelog.content, &new_changes);
            if let Some(stdout) = dry_run {
                writeln!(
                    stdout,
                    "Would add the following to {}:",
                    changelog.path.display()
                )?;
                writeln!(stdout, "{}", &new_changes)?;
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
