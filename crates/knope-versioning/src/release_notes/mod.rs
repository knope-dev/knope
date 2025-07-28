use std::{borrow::Cow, fmt::Write, iter::Peekable};

pub use changelog::Changelog;
pub use config::{CommitFooter, CustomChangeType, SectionName, SectionSource, Sections};
use itertools::Itertools;
pub use release::Release;
use serde::Deserialize;
use time::{OffsetDateTime, macros::format_description};

use crate::{
    Action,
    changes::{Change, ChangeSource},
    package,
    semver::Version,
};

mod changelog;
mod config;
mod release;

/// Defines how release notes are handled for a package.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReleaseNotes {
    pub sections: Sections,
    pub changelog: Option<Changelog>,
    pub change_templates: Vec<ChangeTemplate>,
}

impl ReleaseNotes {
    /// Create new release notes for use in changelogs / forges.
    ///
    /// # Errors
    ///
    /// If the current date can't be formatted
    pub(crate) fn create_release(
        &mut self,
        version: Version,
        changes: &[Change],
        package_name: &package::Name,
    ) -> Result<Vec<Action>, TimeError> {
        let mut notes = String::new();
        for (section_name, sources) in self.sections.iter() {
            let mut changes = changes
                .iter()
                .filter(|change| sources.contains(&change.change_type))
                .sorted()
                .peekable();
            if changes.peek().is_some() {
                if !notes.is_empty() {
                    notes.push_str("\n\n");
                }
                notes.push_str("## ");
                notes.push_str(section_name.as_ref());
                notes.push_str("\n\n");
                write_body(&mut notes, changes, &self.change_templates);
            }
        }

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

fn write_body<'change>(
    out: &mut String,
    changes: Peekable<impl Iterator<Item = &'change Change>>,
    templates: &[ChangeTemplate],
) {
    let mut changes = changes.peekable();
    while let Some(change) = changes.next() {
        write_change(out, change, templates);

        match changes.peek().map(|change| change.details.is_some()) {
            Some(false) => out.push('\n'),
            Some(true) => out.push_str("\n\n"),
            None => (),
        }
    }
}

fn write_change(out: &mut String, change: &Change, templates: &[ChangeTemplate]) {
    for template in templates {
        if template.write(change, out) {
            return;
        }
    }
    if let Some(details) = &change.details {
        write!(out, "### {summary}\n\n{details}", summary = change.summary).unwrap();
    } else {
        write!(out, "- {summary}", summary = change.summary).unwrap();
    }
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ChangeTemplate(Cow<'static, str>);

impl ChangeTemplate {
    const COMMIT_AUTHOR_NAME: &'static str = "$commit_author_name";
    const COMMIT_HASH: &'static str = "$commit_hash";
    const SUMMARY: &'static str = "$summary";
    const DETAILS: &'static str = "$details";

    fn write(&self, change: &Change, out: &mut String) -> bool {
        let mut result = self.0.to_string();
        if result.contains(Self::COMMIT_AUTHOR_NAME) || result.contains(Self::COMMIT_HASH) {
            if let ChangeSource::ConventionalCommit {
                author_name: author,
                hash,
                ..
            } = &change.original_source
            {
                result = result.replace(
                    Self::COMMIT_AUTHOR_NAME,
                    author.as_deref().unwrap_or_default(),
                );
                result = result.replace(Self::COMMIT_HASH, hash.as_deref().unwrap_or_default());
            } else {
                return false;
            }
        }

        if result.contains(Self::DETAILS) {
            if let Some(details) = change.details.as_deref() {
                result = result.replace(Self::DETAILS, details);
            } else {
                return false;
            }
        }

        result = result.replace(Self::SUMMARY, &change.summary);
        out.push_str(&result);

        true
    }
}

impl From<String> for ChangeTemplate {
    fn from(template: String) -> Self {
        Self(Cow::Owned(template))
    }
}

impl From<&'static str> for ChangeTemplate {
    fn from(template: &'static str) -> Self {
        Self(Cow::Borrowed(template))
    }
}

#[cfg(test)]
mod test_release_notes {
    use std::sync::Arc;

    use changesets::UniqueId;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::changes::{ChangeSource, ChangeType};

    #[test]
    fn simple_changes_before_complex() {
        let changes = vec![
            Change {
                change_type: ChangeType::Feature,
                original_source: ChangeSource::ChangeFile(Arc::new(UniqueId::exact(""))),
                summary: "a complex feature".into(),
                details: Some("some details".into()),
            },
            Change {
                change_type: ChangeType::Feature,
                original_source: ChangeSource::ChangeFile(Arc::new(UniqueId::exact(""))),
                summary: "a simple feature".into(),
                details: None,
            },
            Change {
                change_type: ChangeType::Feature,
                original_source: ChangeSource::ConventionalCommit {
                    description: String::new(),
                    author_name: None,
                    hash: None,
                },
                summary: "a super simple feature".into(),
                details: None,
            },
        ];

        let mut actions = ReleaseNotes::create_release(
            &mut ReleaseNotes::default(),
            Version::new(1, 0, 0, None),
            &changes,
            &package::Name::Default,
        )
        .expect("can create release notes");
        assert_eq!(actions.len(), 1);

        let action = actions.pop().unwrap();

        let Action::CreateRelease(release) = action else {
            panic!("expected release action");
        };

        assert_eq!(
            release.notes,
            "## Features\n\n- a simple feature\n- a super simple feature\n\n### a complex feature\n\nsome details"
        );
    }

    #[test]
    fn custom_templates() {
        let change_templates = [
            "* $summary by $commit_author_name ($commit_hash)", // commit-only
            "###### $summary!!! $notAVariable\n\n$details", // Complex change files, should skip #s
            "* $summary",                                   // A fallback that's always applicable
        ]
        .into_iter()
        .map(ChangeTemplate::from)
        .collect_vec();

        let mut release_notes = ReleaseNotes {
            change_templates,
            changelog: Some(Changelog::new(
                "CHANGELOG.md".into(),
                "# My Changelog\n\n## 1.2.3 (previous version)".to_string(),
            )),
            ..ReleaseNotes::default()
        };

        let changes = &[
            Change {
                change_type: ChangeType::Feature,
                original_source: ChangeSource::ChangeFile(Arc::new(UniqueId::exact(""))),
                summary: "a complex feature".to_string(),
                details: Some("some details".into()),
            },
            Change {
                change_type: ChangeType::Feature,
                original_source: ChangeSource::ChangeFile(Arc::new(UniqueId::exact(""))),
                summary: "a simple feature".into(),
                details: None,
            },
            Change {
                change_type: ChangeType::Feature,
                original_source: ChangeSource::ConventionalCommit {
                    description: String::new(),
                    author_name: Some("Sushi".into()),
                    hash: Some("1234".into()),
                },
                summary: "a super simple feature".into(),
                details: None,
            },
        ];

        let mut actions = release_notes
            .create_release(
                Version::new(1, 3, 0, None),
                changes,
                &package::Name::Default,
            )
            .expect("can create release notes");
        let Some(Action::CreateRelease(release)) = actions.pop() else {
            panic!("expected release action");
        };

        assert_eq!(
            release.notes,
            "## Features\n\n* a simple feature\n* a super simple feature by Sushi (1234)\n\n###### a complex feature!!! $notAVariable\n\nsome details"
        );

        let Some(Action::WriteToFile { diff, .. }) = actions.pop() else {
            panic!("expected write changelog action");
        };

        assert!(
            diff.ends_with(
            "\n\n### Features\n\n* a simple feature\n* a super simple feature by Sushi (1234)\n\n####### a complex feature!!! $notAVariable\n\nsome details\n"
            ) // Can't check the date
        );
    }

    #[test]
    fn fall_back_to_built_in_templates() {
        let change_templates = ["* $summary by $commit_author_name"]
            .into_iter()
            .map(ChangeTemplate::from)
            .collect_vec(); // Only applies to commits
        let mut release_notes = ReleaseNotes {
            change_templates,
            ..ReleaseNotes::default()
        };

        let changes = &[
            Change {
                change_type: ChangeType::Feature,
                original_source: ChangeSource::ChangeFile(Arc::new(UniqueId::exact(""))),
                summary: "a complex feature".to_string(),
                details: Some("some details".into()),
            },
            Change {
                change_type: ChangeType::Feature,
                original_source: ChangeSource::ChangeFile(Arc::new(UniqueId::exact(""))),
                summary: "a simple feature".into(),
                details: None,
            },
            Change {
                change_type: ChangeType::Feature,
                original_source: ChangeSource::ConventionalCommit {
                    description: String::new(),
                    author_name: Some("Sushi".into()),
                    hash: None,
                },
                summary: "a super simple feature".into(),
                details: None,
            },
        ];

        let mut actions = release_notes
            .create_release(
                Version::new(1, 3, 0, None),
                changes,
                &package::Name::Default,
            )
            .expect("can create release notes");
        let Some(Action::CreateRelease(release)) = actions.pop() else {
            panic!("expected release action");
        };
        assert_eq!(
            release.notes,
            "## Features\n\n- a simple feature\n* a super simple feature by Sushi\n\n### a complex feature\n\nsome details"
        );
    }
}
