use std::{cmp::Ordering, fmt::Write};

pub use changelog::Changelog;
pub use config::{CommitFooter, CustomChangeType, SectionName, SectionSource, Sections};
use itertools::Itertools;
pub use release::Release;
use time::{OffsetDateTime, macros::format_description};

use crate::{Action, changes::Change, package, semver::Version};

mod changelog;
mod config;
mod release;

/// Defines how release notes are handled for a package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseNotes {
    pub sections: Sections,
    pub changelog: Option<Changelog>,
}

impl ReleaseNotes {
    /// Create new release notes for use in changelogs / forges.
    ///
    /// # Errors
    ///
    /// If the current date can't be formatted
    pub fn create_release(
        &mut self,
        version: Version,
        changes: &[Change],
        package_name: &package::Name,
    ) -> Result<Vec<Action>, TimeError> {
        let mut notes = String::new();
        for (section_name, sources) in self.sections.iter() {
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
            if !changes.is_empty() {
                notes.push_str("\n\n## ");
                notes.push_str(section_name.as_ref());
                notes.push_str("\n\n");
                notes.push_str(&build_body(changes));
            }
        }

        let notes = notes.trim().to_string();
        let release = Release {
            title: release_title(&version)?,
            version,
            notes,
            package_name: package_name.clone(),
        };

        let mut pending_actions = Vec::with_capacity(2);
        if let Some(changelog) = self.changelog.as_mut() {
            let new_changes = changelog.with_release(&release);
            pending_actions.push(Action::WriteToFile {
                path: changelog.path.clone(),
                content: changelog.content.clone(),
                diff: format!("\n{new_changes}\n"),
            });
        }
        pending_actions.push(Action::CreateRelease(release));
        Ok(pending_actions)
    }
}

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
#[error("Failed to format current time")]
#[cfg_attr(
    feature = "miette",
    diagnostic(
        code(release_notes::time_format),
        help(
            "This is probably a bug with knope, please file an issue at https://github.com/knope-dev/knope"
        )
    )
)]
pub struct TimeError(#[from] time::error::Format);

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

fn build_body(changes: Vec<ChangeDescription>) -> String {
    let mut body = String::new();
    let mut changes = changes.into_iter().peekable();
    while let Some(change) = changes.next() {
        match change {
            ChangeDescription::Simple(summary) => {
                write!(&mut body, "- {summary}").ok();
            }
            ChangeDescription::Complex(summary, details) => {
                write!(&mut body, "### {summary}\n\n{details}").ok();
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

/// Create the title of a new release with no Markdown header level.
///
/// # Errors
///
/// If the current date can't be formatted
fn release_title(version: &Version) -> Result<String, TimeError> {
    let format = format_description!("[year]-[month]-[day]");
    let date_str = OffsetDateTime::now_utc().date().format(&format)?;
    Ok(format!("{version} ({date_str})"))
}

#[cfg(test)]
mod test_change_description {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::changes::{ChangeSource, ChangeType};

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
