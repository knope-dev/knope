use std::{cmp::Ordering, fmt::Display, mem::swap, path::PathBuf, str::FromStr};

use itertools::Itertools;
use knope_versioning::{GoVersioning, Version};
use miette::Diagnostic;
use thiserror::Error;
use time::{macros::format_description, Date, OffsetDateTime};

use super::{Change, Package, TimeError};
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
    pub(crate) fn get_release(
        &self,
        version: &Version,
        package: Option<knope_versioning::Package>,
        go_versioning: GoVersioning,
    ) -> Result<Option<Release>, ParseError> {
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
        let additional_tags = package
            .map(|pkg| pkg.set_version(&version, go_versioning).unwrap_or_default())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|action| match action {
                knope_versioning::Action::AddTag { tag } => Some(tag),
                knope_versioning::Action::WriteToFile { .. } => None,
            })
            .collect();
        Ok(Some(Release {
            version,
            date,
            sections,
            header_level,
            additional_tags,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Release {
    pub(crate) version: Version,
    pub(crate) date: Option<Date>,
    pub(crate) sections: Option<Vec<Section>>,
    /// The expected header level of the release title (# or ##).
    ///
    /// Content within is written expecting that the release title will be written at this level
    header_level: HeaderLevel,
    /// Any tags that should be added for the sake of the versioned files (specifically `go.mod`s)
    /// This doesn't include the package-level tags, since those will get added by GitHub/Gitea
    /// sometimes.
    pub(crate) additional_tags: Vec<String>,
}
impl Release {
    pub(crate) fn new(
        version: Version,
        changes: &[Change],
        changelog_sections: &ChangelogSections,
        header_level: HeaderLevel,
        additional_tags: Vec<String>,
    ) -> Self {
        let sections = changelog_sections
            .iter()
            .filter_map(|(section_name, sources)| {
                let changes = changes
                    .iter()
                    .filter_map(|change| {
                        if sources.contains(&change.change_type()) {
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
                        body: build_body(changes, header_level),
                    })
                }
            })
            .collect_vec();

        let sections = (!sections.is_empty()).then_some(sections);
        let date = Some(OffsetDateTime::now_utc().date());
        Self {
            version,
            date,
            sections,
            header_level,
            additional_tags,
        }
    }

    pub(crate) fn empty(version: Version, additional_tags: Vec<String>) -> Self {
        Self {
            version,
            date: Some(OffsetDateTime::now_utc().date()),
            sections: None,
            header_level: HeaderLevel::H2,
            additional_tags,
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
    pub(crate) fn body(&self) -> Option<String> {
        let sections = self.sections.as_ref()?;
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
        match change {
            Change::ConventionalCommit(commit) => Self::Simple(commit.message.clone()),
            Change::ChangeSet(changeset) => {
                let mut lines = changeset
                    .summary
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

#[cfg(test)]
mod test_change_description {
    use changesets::{PackageChange, UniqueId};
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::step::releases::{conventional_commits::ConventionalCommit, ChangeType};

    #[test]
    fn conventional_commit() {
        let change = Change::ConventionalCommit(ConventionalCommit {
            change_type: ChangeType::Feature,
            original_source: String::new(),
            message: "a feature".to_string(),
        });
        let description = ChangeDescription::from(&change);
        assert_eq!(
            description,
            ChangeDescription::Simple("a feature".to_string())
        );
    }

    #[test]
    fn simple_changeset() {
        let change = Change::ChangeSet(PackageChange {
            unique_id: UniqueId::from(""),
            change_type: changesets::ChangeType::Minor,
            summary: "# a feature\n\n\n\n".to_string(),
        });
        let description = ChangeDescription::from(&change);
        assert_eq!(
            description,
            ChangeDescription::Simple("a feature".to_string())
        );
    }

    #[test]
    fn complex_changeset() {
        let change = Change::ChangeSet(PackageChange {
            unique_id: UniqueId::from(""),
            change_type: changesets::ChangeType::Minor,
            summary: "# a feature\n\nwith details\n\n- first\n- second".to_string(),
        });
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

impl Section {
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
        let mut additional_tags = Vec::new();
        swap(&mut self.pending_tags, &mut additional_tags);
        let release = Release::new(
            version,
            &self.pending_changes,
            &self.changelog_sections,
            self.changelog
                .as_ref()
                .map_or(HeaderLevel::H2, |it| it.section_header_level),
            additional_tags,
        );

        if let Some(changelog) = self.changelog.as_mut() {
            changelog.add_release(&release, dry_run)?;
        }

        Ok(release)
    }
}
