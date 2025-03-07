use std::{fmt::Display, str::FromStr};

use itertools::Itertools;
use relative_path::RelativePathBuf;
use thiserror::Error;
use time::{Date, macros::format_description};

use crate::{package, release_notes::Release, semver::Version};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Changelog {
    /// The path to the CHANGELOG file
    pub path: RelativePathBuf,
    /// The content that's been written to `path`
    pub content: String,
    /// The header level of the title of each release (the version + date)
    release_header_level: HeaderLevel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HeaderLevel {
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

impl Changelog {
    #[must_use]
    pub fn new(path: RelativePathBuf, content: String) -> Self {
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
        Changelog {
            path,
            content,
            release_header_level,
        }
    }

    /// Find a release matching `version`, if any, within the changelog.
    #[must_use]
    pub fn get_release(&self, version: &Version, package_name: &package::Name) -> Option<Release> {
        let expected_header_start = format!(
            "{release_header_level} {version}",
            release_header_level = self.release_header_level
        );

        let mut lines = self.content.lines();
        let (title, version) = loop {
            let line = lines.next()?;
            if !line.starts_with(&expected_header_start) {
                continue;
            }
            let Ok((header_level, title_version, _)) = parse_title(line) else {
                continue;
            };
            if header_level == self.release_header_level && *version == title_version {
                break (
                    // Release titles should not be markdown formatted
                    line.trim_start_matches('#').trim().to_string(),
                    title_version,
                );
            }
        };
        let notes = lines
            .take_while(|line| {
                !line.starts_with(&format!(
                    // Next version
                    "{release_header_level} ",
                    release_header_level = self.release_header_level
                ))
            })
            .map(|line| match self.release_header_level {
                HeaderLevel::H1 => line,
                HeaderLevel::H2 => reduce_header_level(line),
            })
            .join("\n");
        (!notes.is_empty()).then_some(Release {
            title,
            version,
            notes,
            package_name: package_name.clone(),
        })
    }

    /// Update `self.content` with the new release, return the diff being applied.
    #[must_use]
    pub fn with_release(&mut self, release: &Release) -> String {
        let mut not_written = true;
        let new_changes = format!(
            "{header_level} {title}\n\n{body}",
            header_level = self.release_header_level,
            title = release.title,
            body = release
                .notes
                .lines()
                .map(|line| {
                    // Release notes are at H1, we need to format them properly for this changelog
                    if line.starts_with('#') && self.release_header_level == HeaderLevel::H2 {
                        format!("#{line}")
                    } else {
                        line.to_string()
                    }
                })
                .join("\n")
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
        new_changes
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

#[derive(Clone, Debug, Eq, PartialEq, Error)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum ParseError {
    #[error("Missing version")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code = "changelog::missing_version",
            help = "The expected changelog format is very particular, a release title must start with the
            semantic version immediately after the header level. For example: `## 0.1.0 - 2020-12-25"
        )
    )]
    MissingVersion,
    #[error("Bad header level")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code = "changelog::header_level",
            help = "The expected changelog format is very particular, a release title be header level 1
            (#) or 2 (##). For example: `## 0.1.0 - 2020-12-25"
        )
    )]
    HeaderLevel,
}
