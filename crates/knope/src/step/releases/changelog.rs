use std::{cmp::Ordering, fmt::Display, str::FromStr};

use itertools::Itertools;
use knope_versioning::{changelog::Sections, changes::Change, semver::Version};
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use thiserror::Error;
use time::{macros::format_description, Date, OffsetDateTime};

use super::TimeError;
use crate::fs;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Changelog {
    /// The path to the CHANGELOG file
    pub(crate) path: RelativePathBuf,
    /// The content that's been written to `path`
    pub(crate) content: String,
    pub(crate) release_header_level: HeaderLevel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HeaderLevel {
    H1,
    H2,
}

impl HeaderLevel {
    const fn as_str(self) -> &'static str {
        match self {
            Self::H1 => "#",
            Self::H2 => "##",
        }
    }
}

impl Display for HeaderLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<RelativePathBuf> for Changelog {
    type Error = Error;

    fn try_from(path: RelativePathBuf) -> Result<Self, Self::Error> {
        let path_buf = path.to_path("");
        let content = if path_buf.exists() {
            fs::read_to_string(path_buf)?
        } else {
            String::new()
        };
        let release_header_level = content
            .lines()
            .filter(|line| line.starts_with('#'))
            .nth(1)
            .and_then(|header| {
                if header.starts_with("##") {
                    Some(HeaderLevel::H2)
                } else if header.starts_with('#') {
                    Some(HeaderLevel::H1)
                } else {
                    None
                }
            })
            .unwrap_or(HeaderLevel::H2);
        Ok(Self {
            path,
            content,
            release_header_level,
        })
    }
}

impl Changelog {
    /// Find a release matching `version`, if any, within the changelog.
    pub(crate) fn get_release(&self, version: &Version) -> Option<Release> {
        let expected_header_start = format!(
            "{release_header_level} {version}",
            release_header_level = self.release_header_level
        );

        let mut lines = self.content.lines();
        let title = loop {
            let line = lines.next()?;
            if !line.starts_with(&expected_header_start) {
                continue;
            }
            let Ok((header_level, title_version, _)) = parse_title(line) else {
                continue;
            };
            if header_level == self.release_header_level && *version == title_version {
                break line.to_string();
            }
        };
        let body = lines
            .take_while(|line| {
                !line.starts_with(&format!(
                    // Next version
                    "{release_header_level} ",
                    release_header_level = self.release_header_level
                ))
            })
            .join("\n");
        (!body.is_empty()).then_some(Release {
            title,
            body,
            header_level: self.release_header_level,
        })
    }

    /// Update `self.content` with the new release.
    pub(crate) fn with_release(mut self, release: &Release) -> (Self, String) {
        let mut not_written = true;
        let new_changes = format!(
            "{title}\n\n{body}",
            title = release.title,
            body = release.body
        );
        let mut new_content = String::with_capacity(self.content.len() + new_changes.len());

        for line in self.content.lines() {
            if not_written && parse_title(line).is_ok() {
                // Insert new changes before the next release in the changelog
                new_content.push_str(&new_changes);
                new_content.push_str("\n\n");
                not_written = false;
            }
            new_content.push_str(line);
            new_content.push('\n');
        }

        if not_written {
            new_content.push_str(&new_changes);
        }

        if (self.content.ends_with('\n') || self.content.is_empty()) && !new_content.ends_with('\n')
        {
            // Preserve white space at end of file
            new_content.push('\n');
        }

        self.content = new_content;
        (self, new_changes)
    }
}

fn parse_title(title: &str) -> Result<(HeaderLevel, Version, Option<Date>), ParseError> {
    let mut parts = title.split_ascii_whitespace();
    let header_level = match parts.next() {
        Some("##") => HeaderLevel::H2,
        Some("#") => HeaderLevel::H1,
        _ => return Err(ParseError::HeaderLevel),
    };
    let version = parts.next().ok_or(ParseError::MissingVersion)?;
    let version = Version::from_str(version).map_err(|_| ParseError::MissingVersion)?;
    let mut date = None;
    for part in parts {
        let part = part.trim_start_matches('(').trim_end_matches(')');
        date = Date::parse(part, format_description!("[year]-[month]-[day]")).ok();
        if date.is_some() {
            break;
        }
    }
    Ok((header_level, version, date))
}

fn reduce_header_level(line: &str) -> &str {
    if line.starts_with("##") {
        #[allow(clippy::indexing_slicing)] // Just checked len above
        &line[1..] // Reduce header level by one
    } else {
        line
    }
}

/// The Markdown body for this release.
///
/// Since [`Self::header_level`] refers to the header level
/// of the title, each section in this body is one level lower. More concretely, if
/// [`Self::header_level`] is [`HeaderLevel::H1`], the body will look like this:
///
/// ```markdown
/// ## Breaking changes
///
/// - a change
///
/// ### a complex change
///
/// with details
///
/// ## Features
///
/// - a feature
///
/// ### a complex feature
///
/// with details
/// ```
///
/// If [`Self::header_level`] is [`HeaderLevel::H2`], the body will look like this:
///
/// ```markdown
/// ### Breaking changes
///
/// - a change
///
/// #### a complex change
///
/// with details
///
/// ### Features
///
/// - a feature
///
/// #### a complex feature
///
/// with details
/// ```
///
/// GitHub releases _always_ use the [`HeaderLevel::H1`] format, so they call [`Self::body_at_h1`]
/// which is like this function, but with optional conversion.
pub(crate) fn sections_to_markdown<T: Iterator<Item = Section>>(
    header_level: HeaderLevel,
    sections: T,
) -> String {
    let mut res = String::new();
    for Section { title, body } in sections {
        res.push_str(&format!("\n\n{header_level}# {title}\n\n{body}",));
    }
    let res = res.trim().to_string();
    res.trim().to_string()
}

/// The details of a single release (version) within a changelog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Release {
    pub(crate) title: String,
    pub(crate) body: String,
    header_level: HeaderLevel,
}

impl Release {
    pub(crate) fn new(
        version: &Version,
        changes: &[Change],
        changelog_sections: &Sections,
        release_header_level: HeaderLevel,
    ) -> Result<Self, TimeError> {
        let sections = changelog_sections
            .iter()
            .filter_map(|(section_name, sources)| {
                let changes = changes
                    .iter()
                    .filter_map(|change| {
                        if sources.contains(&change.change_type) {
                            Some(ChangeDescription::from(change))
                        } else {
                            None
                        }
                    })
                    .sorted()
                    .collect_vec();
                if changes.is_empty() {
                    None
                } else {
                    Some(Section {
                        title: section_name.to_string(),
                        body: build_body(changes, release_header_level),
                    })
                }
            })
            .collect_vec();

        Ok(Self {
            title: release_title(version, Some(release_header_level), true)?,
            body: sections_to_markdown(release_header_level, sections.iter().cloned()),
            header_level: release_header_level,
        })
    }

    pub(crate) fn body_at_h1(self) -> String {
        match self.header_level {
            HeaderLevel::H1 => self.body,
            HeaderLevel::H2 => self.body.lines().map(reduce_header_level).join("\n"),
        }
    }
}

/// Create the title of a new release.
///
/// If a `markdown_header` is passed, the title will be formatted as a Markdown header.
pub(crate) fn release_title(
    version: &Version,
    markdown_header: Option<HeaderLevel>,
    add_date: bool,
) -> Result<String, TimeError> {
    let mut title = if let Some(markdown_header) = markdown_header {
        format!("{} ", markdown_header.as_str())
    } else {
        String::new()
    };
    title.push_str(&version.to_string());
    if add_date {
        let format = format_description!("[year]-[month]-[day]");
        let date_str = OffsetDateTime::now_utc().date().format(&format)?;
        title.push_str(" (");
        title.push_str(&date_str);
        title.push(')');
    };
    Ok(title)
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ChangeDescription {
    Simple(String),
    Complex(String, String),
}

impl Ord for ChangeDescription {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Simple(_), Self::Complex(_, _)) => Ordering::Less,
            (Self::Complex(_, _), Self::Simple(_)) => Ordering::Greater,
            _ => Ordering::Equal,
        }
    }
}

impl PartialOrd for ChangeDescription {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<&Change> for ChangeDescription {
    fn from(change: &Change) -> Self {
        let mut lines = change
            .description
            .trim()
            .lines()
            .skip_while(|it| it.is_empty());
        let summary: String = lines
            .next()
            .unwrap_or_default()
            .chars()
            .skip_while(|it| *it == '#' || *it == ' ')
            .collect();
        let body: String = lines.skip_while(|it| it.is_empty()).join("\n");
        if body.is_empty() {
            Self::Simple(summary)
        } else {
            Self::Complex(summary, body)
        }
    }
}

fn build_body(changes: Vec<ChangeDescription>, header_level: HeaderLevel) -> String {
    let mut body = String::new();
    let mut changes = changes.into_iter().peekable();
    while let Some(change) = changes.next() {
        match change {
            ChangeDescription::Simple(summary) => {
                body.push_str(&format!("- {summary}"));
            }
            ChangeDescription::Complex(summary, details) => {
                body.push_str(&format!("{header_level}## {summary}\n\n{details}"));
            }
        }
        match changes.peek() {
            Some(ChangeDescription::Simple(_)) => body.push('\n'),
            Some(ChangeDescription::Complex(_, _)) => body.push_str("\n\n"),
            None => (),
        }
    }
    body
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test_parse_title {
    use time::macros::date;

    use super::*;

    #[test]
    fn no_date() {
        let title = "## 0.1.2";
        let (header_level, version, date) = parse_title(title).unwrap();
        assert_eq!(header_level, HeaderLevel::H2);
        assert_eq!(version, Version::new(0, 1, 2, None));
        assert!(date.is_none());
    }

    #[test]
    fn with_date() {
        let title = "## 0.1.2 (2023-05-02)";
        let (header_level, version, date) = parse_title(title).unwrap();
        assert_eq!(header_level, HeaderLevel::H2);
        assert_eq!(version, Version::new(0, 1, 2, None));
        assert_eq!(date, Some(date!(2023 - 05 - 02)));
    }

    #[test]
    fn no_version() {
        let title = "## 2023-05-02";
        let result = parse_title(title);
        assert!(result.is_err());
    }

    #[test]
    fn bad_version() {
        let title = "## sad";
        let result = parse_title(title);
        assert!(result.is_err());
    }

    #[test]
    fn h1() {
        let title = "# 0.1.2 (2023-05-02)";
        let (header_level, version, date) = parse_title(title).unwrap();
        assert_eq!(header_level, HeaderLevel::H1);
        assert_eq!(version, Version::new(0, 1, 2, None));
        assert_eq!(date, Some(date!(2023 - 05 - 02)));
    }
}

#[cfg(test)]
mod test_change_description {
    use knope_versioning::changes::{ChangeSource, ChangeType};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn conventional_commit() {
        let change = Change {
            change_type: ChangeType::Feature,
            original_source: ChangeSource::ConventionalCommit(String::new()),
            description: "a feature".into(),
        };
        let description = ChangeDescription::from(&change);
        assert_eq!(
            description,
            ChangeDescription::Simple("a feature".to_string())
        );
    }

    #[test]
    fn simple_changeset() {
        let change = Change {
            change_type: ChangeType::Feature,
            original_source: ChangeSource::ConventionalCommit(String::new()),
            description: "# a feature\n\n\n\n".into(),
        };
        let description = ChangeDescription::from(&change);
        assert_eq!(
            description,
            ChangeDescription::Simple("a feature".to_string())
        );
    }

    #[test]
    fn complex_changeset() {
        let change = Change {
            original_source: ChangeSource::ConventionalCommit(String::new()),
            change_type: ChangeType::Feature,
            description: "# a feature\n\nwith details\n\n- first\n- second".into(),
        };
        let description = ChangeDescription::from(&change);
        assert_eq!(
            description,
            ChangeDescription::Complex(
                "a feature".to_string(),
                "with details\n\n- first\n- second".to_string()
            )
        );
    }
}

#[derive(Clone, Debug, Diagnostic, Eq, PartialEq, thiserror::Error)]
pub(crate) enum ParseError {
    #[error("Missing version")]
    #[diagnostic(
        code = "changelog::missing_version",
        help = "The expected changelog format is very particular, a release title must start with the 
            semantic version immediately after the header level. For example: `## 0.1.0 - 2020-12-25"
    )]
    MissingVersion,
    #[error("Bad header level")]
    #[diagnostic(
        code = "changelog::header_level",
        help = "The expected changelog format is very particular, a release title be header level 1 
            (#) or 2 (##). For example: `## 0.1.0 - 2020-12-25"
    )]
    HeaderLevel,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Section {
    /// The title of the section _without_ any header level (e.g., "Breaking changes" not "### Breaking changes")
    title: String,
    /// The Markdown body of the section including any headers.
    body: String,
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    TimeError(#[from] TimeError),
}
