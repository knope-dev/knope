use std::{fmt::Display, path::PathBuf, str::FromStr};

use indexmap::IndexMap;
use itertools::Itertools;
use knope_versioning::Version;
use miette::Diagnostic;
use thiserror::Error;
use time::{macros::format_description, Date, OffsetDateTime};

use super::{Change, ChangeType, ChangelogSectionSource, Package, TimeError};
use crate::{dry_run::DryRun, fs, step::releases::package::ChangelogSections};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Changelog {
    /// The path to the CHANGELOG file
    pub(crate) path: PathBuf,
    /// The content that has been written to `path`
    pub(crate) content: String,
    section_header_level: HeaderLevel,
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

impl TryFrom<PathBuf> for Changelog {
    type Error = Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let content = if path.exists() {
            fs::read_to_string(&path)?
        } else {
            String::new()
        };
        let section_header_level = content
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
            section_header_level,
        })
    }
}

impl Changelog {
    pub(crate) fn get_release(&self, version: &Version) -> Result<Option<Release>, ParseError> {
        let section_header_level = self.section_header_level.as_str();
        let expected_header_start = format!("{section_header_level} {version}");
        let mut content_starting_with_first_release = self
            .content
            .lines()
            .skip_while(|line| !line.starts_with(&expected_header_start));

        let Some(title) = content_starting_with_first_release.next().map(String::from) else {
            return Ok(None);
        };
        let (header_level, version, date) = Release::parse_title(&title)?;

        let release_sections = content_starting_with_first_release.take_while(
            |line| !line.starts_with(&format!("{section_header_level} ")), // Next version
        );
        let sections = Some(Section::from_lines(
            release_sections,
            &format!("{section_header_level}#"),
        ));
        Ok(Some(Release {
            version,
            date,
            sections,
            header_level,
        }))
    }

    fn add_release(&mut self, release: &Release, dry_run: DryRun) -> Result<(), Error> {
        let mut changelog = String::new();
        let mut not_written = true;
        let Some(new_changes) = release.body() else {
            return Ok(());
        };
        let new_changes = format!(
            "{title}\n\n{new_changes}",
            title = release.title(true, true)?,
        );

        for line in self.content.lines() {
            if not_written && Release::parse_title(line).is_ok() {
                // Insert new changes before the next release in the changelog
                changelog.push_str(&new_changes);
                changelog.push_str("\n\n");
                not_written = false;
            }
            changelog.push_str(line);
            changelog.push('\n');
        }

        if not_written {
            changelog.push_str(&new_changes);
        }

        if (self.content.ends_with('\n') || self.content.is_empty()) && !changelog.ends_with('\n') {
            // Preserve white space at end of file
            changelog.push('\n');
        }

        self.content = changelog;
        fs::write(
            dry_run,
            &format!("\n{new_changes}\n"),
            &self.path,
            &self.content,
        )
        .map_err(Error::Fs)
    }
}

#[cfg(test)]
mod test_get_release {
    use std::path::PathBuf;

    use pretty_assertions::assert_eq;
    use time::macros::date;

    use super::*;
    use crate::step::releases::changelog::HeaderLevel;

    const CONTENT: &str = r"
# Changelog

Hey ya'll this is a changelog

## 0.1.2 2023-05-02

### Features
- Blah

## 0.1.1

### Fixes

#### it's fixex!

Now with more detail!

## 0.0.1
Initial release
";

    #[test]
    fn first_section() {
        let changelog = Changelog {
            path: PathBuf::default(),
            content: CONTENT.to_string(),
            section_header_level: HeaderLevel::H2,
        };

        let section = changelog
            .get_release(&Version::new(0, 1, 2, None))
            .unwrap()
            .unwrap();
        let expected = Release {
            version: Version::new(0, 1, 2, None),
            date: Some(date!(2023 - 05 - 02)),
            sections: Some(vec![Section {
                title: "Features".to_string(),
                body: "- Blah".to_string(),
            }]),
            header_level: HeaderLevel::H2,
        };
        assert_eq!(section, expected);
    }

    #[test]
    fn middle_section() {
        let changelog = Changelog {
            path: PathBuf::default(),
            content: CONTENT.to_string(),
            section_header_level: HeaderLevel::H2,
        };

        let section = changelog
            .get_release(&Version::new(0, 1, 1, None))
            .unwrap()
            .unwrap();
        let expected = Release {
            version: Version::new(0, 1, 1, None),
            date: None,
            sections: Some(vec![Section {
                title: "Fixes".to_string(),
                body: "#### it's fixex!\n\nNow with more detail!".to_string(),
            }]),
            header_level: HeaderLevel::H2,
        };
        assert_eq!(section, expected);
    }

    #[test]
    fn no_section() {
        let changelog = Changelog {
            path: PathBuf::default(),
            content: CONTENT.to_string(),
            section_header_level: HeaderLevel::H2,
        };

        let section = changelog.get_release(&Version::new(0, 1, 0, None)).unwrap();
        assert!(section.is_none());
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Release {
    pub(crate) version: Version,
    pub(crate) date: Option<Date>,
    pub(crate) sections: Option<Vec<Section>>,
    /// The expected header level of the release title (# or ##).
    ///
    /// Content within is written expecting that the release title will be written at this level
    header_level: HeaderLevel,
}
impl Release {
    pub(crate) fn new(
        version: Version,
        changes: &[Change],
        changelog_sections: &ChangelogSections,
        header_level: HeaderLevel,
    ) -> Self {
        let mut sections: IndexMap<String, Section> = changelog_sections
            .values()
            .map(|name| (name.to_string(), Section::new(name.to_string())))
            .collect();
        let breaking_source = ChangelogSectionSource::CustomChangeType("major".into());
        let feature_source = ChangelogSectionSource::CustomChangeType("minor".into());
        let fix_source = ChangelogSectionSource::CustomChangeType("patch".into());

        for change in changes {
            let source = match &change.change_type() {
                ChangeType::Breaking => breaking_source.clone(),
                ChangeType::Feature => feature_source.clone(),
                ChangeType::Fix => fix_source.clone(),
                ChangeType::Custom(source) => source.clone(),
            };
            let section = changelog_sections
                .get(&source)
                .and_then(|name| sections.get_mut(name.as_ref()));
            if let Some(section) = section {
                let summary = change.summary();
                // Changesets come with a baked in header, replace it with our own
                let summary: String = summary
                    .chars()
                    .skip_while(|it| *it == '#' || *it == ' ')
                    .collect();
                if !section.body.is_empty() {
                    section.body.push_str("\n\n");
                }
                section
                    .body
                    .push_str(&format!("{header_level}## {summary}"));
            }
        }

        let sections = sections
            .into_values()
            .filter(|section| !section.body.is_empty())
            .collect_vec();
        let sections = (!sections.is_empty()).then_some(sections);
        let date = Some(OffsetDateTime::now_utc().date());
        Self {
            version,
            date,
            sections,
            header_level,
        }
    }

    pub(crate) fn empty(version: Version) -> Self {
        Self {
            version,
            date: Some(OffsetDateTime::now_utc().date()),
            sections: None,
            header_level: HeaderLevel::H2,
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

    /// The Markdown body for this release.
    ///
    /// Since [`Self::header_level`] refers to the header level
    /// of the title, each section in this body is one level lower. More concretely, if
    /// [`Self::header_level`] is [`HeaderLevel::H1`], the body will look like this:
    ///
    /// ```markdown
    /// ## Breaking changes
    ///
    /// ### a change
    ///
    /// ## Features
    ///
    /// ### a feature
    ///
    /// ### another feature
    /// ```
    ///
    /// If [`Self::header_level`] is [`HeaderLevel::H2`], the body will look like this:
    ///
    /// ```markdown
    /// ### Breaking changes
    ///
    /// #### a change
    ///
    /// ### Features
    ///
    /// #### a feature
    ///
    /// #### another feature
    /// ```
    ///
    /// GitHub releases _always_ use the [`HeaderLevel::H1`] format, so they call [`Self::body_at_h1`]
    /// which is like this function, but with optional conversion.
    pub(crate) fn body(&self) -> Option<String> {
        let Some(sections) = self.sections.as_ref() else {
            return None;
        };
        let mut res = String::new();
        for section in sections {
            res.push_str(&format!(
                "\n\n{header_level}# {title}\n\n{body}",
                header_level = self.header_level,
                title = section.title,
                body = section.body
            ));
        }
        let res = res.trim().to_string();
        Some(res.trim().to_string())
    }

    /// Like [`Self::body`], but if this release is not [`HeaderValue::H1`], the body is modified
    /// to be at that level (so sections are `##` and subsections are `###`).
    pub(crate) fn body_at_h1(&self) -> Option<String> {
        if self.header_level == HeaderLevel::H1 {
            return self.body();
        }
        let mut adjusted = self.clone();
        adjusted.header_level = HeaderLevel::H1;
        adjusted.sections = adjusted.sections.map(|sections| {
            sections
                .into_iter()
                .map(|mut section| {
                    section.body = section
                        .body
                        .lines()
                        .map(|line| {
                            if line.starts_with("##") {
                                #[allow(clippy::indexing_slicing)] // Just checked len above
                                &line[1..] // Reduce header level by one
                            } else {
                                line
                            }
                        })
                        .collect_vec()
                        .join("\n");
                    section
                })
                .collect()
        });
        adjusted.body()
    }

    /// The title of the release, which is either the version number or the version number and date.
    ///
    /// If `markdown` is true, the title will be formatted as a Markdown header using `self.header_level`
    pub(crate) fn title(&self, markdown: bool, add_date: bool) -> Result<String, TimeError> {
        let mut title = if markdown {
            format!("{} ", self.header_level.as_str())
        } else {
            String::new()
        };
        title.push_str(&self.version.to_string());
        let mut date = self.date;
        if add_date {
            date = date.or_else(|| Some(OffsetDateTime::now_utc().date()));
        }
        if let Some(date) = &date {
            let format = format_description!("[year]-[month]-[day]");
            let date_str = date.format(&format)?;
            title.push_str(" (");
            title.push_str(&date_str);
            title.push(')');
        };
        Ok(title)
    }
}

#[cfg(test)]
mod test_parse_title {
    use time::macros::date;

    use super::Release;

    #[test]
    fn no_date() {
        let title = "## 0.1.2";
        let (header_level, version, date) = Release::parse_title(title).unwrap();
        assert_eq!(header_level, super::HeaderLevel::H2);
        assert_eq!(version, super::Version::new(0, 1, 2, None));
        assert!(date.is_none());
    }

    #[test]
    fn with_date() {
        let title = "## 0.1.2 (2023-05-02)";
        let (header_level, version, date) = Release::parse_title(title).unwrap();
        assert_eq!(header_level, super::HeaderLevel::H2);
        assert_eq!(version, super::Version::new(0, 1, 2, None));
        assert_eq!(date, Some(date!(2023 - 05 - 02)));
    }

    #[test]
    fn no_version() {
        let title = "## 2023-05-02";
        let result = Release::parse_title(title);
        assert!(result.is_err());
    }

    #[test]
    fn bad_version() {
        let title = "## sad";
        let result = Release::parse_title(title);
        assert!(result.is_err());
    }

    #[test]
    fn h1() {
        let title = "# 0.1.2 (2023-05-02)";
        let (header_level, version, date) = Release::parse_title(title).unwrap();
        assert_eq!(header_level, super::HeaderLevel::H1);
        assert_eq!(version, super::Version::new(0, 1, 2, None));
        assert_eq!(date, Some(date!(2023 - 05 - 02)));
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

impl Section {
    fn new<Title: Into<String>>(title: Title) -> Self {
        Self {
            title: title.into(),
            body: String::new(),
        }
    }
    fn from_lines<'a, 'b, I: Iterator<Item = &'a str>>(
        lines: I,
        header_level: &'b str,
    ) -> Vec<Self> {
        let mut sections = Vec::new();
        let mut lines = lines.peekable();
        let header_start = format!("{header_level} ");
        while let Some(line) = lines.next() {
            if line.starts_with(&header_start) {
                let title = line.trim_start_matches(&header_start).trim().to_string();
                let body = lines
                    .peeking_take_while(|line| !line.starts_with(&header_start))
                    .map(str::trim)
                    .join("\n");
                let body = body.trim().to_string();
                sections.push(Self { title, body });
            }
        }
        sections
    }
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

impl Package {
    /// Adds content from `release` to `Self::changelog` if it exists.
    pub(crate) fn write_changelog(
        &mut self,
        version: Version,
        dry_run: DryRun,
    ) -> Result<Release, Error> {
        let release = Release::new(
            version,
            &self.pending_changes,
            &self.changelog_sections,
            self.changelog
                .as_ref()
                .map_or(HeaderLevel::H2, |it| it.section_header_level),
        );

        if let Some(changelog) = self.changelog.as_mut() {
            changelog.add_release(&release, dry_run)?;
        }

        Ok(release)
    }
}
