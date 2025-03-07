use git_conventional::{Commit, Footer, Type};
use tracing::debug;

use super::{Change, ChangeSource, ChangeType};
use crate::release_notes::Sections;

/// Try to parse each commit message as a [conventional commit](https://www.conventionalcommits.org/).
///
/// # Filtering
///
/// 1. If the commit message doesn't follow the conventional commit format, it is ignored.
/// 2. For non-standard change types, only those included will be considered.
/// 3. For non-standard footers, only those included will be considered.
pub(crate) fn changes_from_commit_messages<'a, Message: AsRef<str>>(
    commit_messages: &'a [Message],
    scopes: Option<&'a Vec<String>>,
    changelog_sections: &'a Sections,
) -> impl Iterator<Item = Change> + 'a {
    if let Some(scopes) = scopes {
        debug!("Only checking commits with scopes: {scopes:?}");
    }
    commit_messages.iter().flat_map(move |message| {
        changes_from_commit_message(message.as_ref(), scopes, changelog_sections).into_iter()
    })
}

fn changes_from_commit_message(
    commit_message: &str,
    scopes: Option<&Vec<String>>,
    changelog_sections: &Sections,
) -> Vec<Change> {
    let Some(commit) = Commit::parse(commit_message.trim()).ok() else {
        return Vec::new();
    };
    let mut has_breaking_footer = false;
    let commit_summary = format_commit_summary(&commit);

    if let Some(commit_scope) = commit.scope() {
        if let Some(scopes) = scopes {
            if !scopes
                .iter()
                .any(|s| s.eq_ignore_ascii_case(commit_scope.as_str()))
            {
                return Vec::new();
            }
        }
    }

    let mut changes = Vec::new();
    for footer in commit.footers() {
        if footer.breaking() {
            has_breaking_footer = true;
        } else if !changelog_sections.contains_footer(footer) {
            continue;
        }
        changes.push(Change {
            change_type: footer.token().into(),
            description: footer.value().into(),
            original_source: ChangeSource::ConventionalCommit(format_commit_footer(
                &commit_summary,
                footer,
            )),
        });
    }

    let commit_description_change_type = if commit.breaking() && !has_breaking_footer {
        ChangeType::Breaking
    } else if commit.type_() == Type::FEAT {
        ChangeType::Feature
    } else if commit.type_() == Type::FIX {
        ChangeType::Fix
    } else {
        return changes; // The commit description isn't a change itself, only (maybe) footers were.
    };

    changes.push(Change {
        change_type: commit_description_change_type,
        description: commit.description().into(),
        original_source: ChangeSource::ConventionalCommit(commit_summary),
    });

    changes
}

fn format_commit_summary(commit: &Commit) -> String {
    let commit_scope = commit
        .scope()
        .map(|s| s.to_string())
        .map(|it| format!("({it})"))
        .unwrap_or_default();
    let bang = if commit.breaking() {
        commit
            .footers()
            .iter()
            .find(|it| it.breaking())
            .map_or_else(|| "!", |_| "")
    } else {
        ""
    };
    format!(
        "{commit_type}{commit_scope}{bang}: {summary}",
        commit_type = commit.type_(),
        summary = commit.description()
    )
}

fn format_commit_footer(commit_summary: &str, footer: &Footer) -> String {
    format!(
        "{commit_summary}\n\tContaining footer {}{} {}",
        footer.token(),
        footer.separator(),
        footer.value()
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use itertools::Itertools;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{
        changes::ChangeSource,
        release_notes::{SectionSource, Sections},
    };

    #[test]
    fn commit_types() {
        let commits = &[
            "fix: a bug",
            "fix!: a breaking bug fix",
            "feat!: add a feature",
            "feat: add another feature",
        ];
        let changes =
            changes_from_commit_messages(commits, None, &Sections::default()).collect_vec();
        assert_eq!(
            changes,
            vec![
                Change {
                    change_type: ChangeType::Fix,
                    description: "a bug".into(),
                    original_source: ChangeSource::ConventionalCommit(String::from("fix: a bug")),
                },
                Change {
                    change_type: ChangeType::Breaking,
                    description: "a breaking bug fix".into(),
                    original_source: ChangeSource::ConventionalCommit(String::from(
                        "fix!: a breaking bug fix"
                    )),
                },
                Change {
                    change_type: ChangeType::Breaking,
                    description: "add a feature".into(),
                    original_source: ChangeSource::ConventionalCommit(String::from(
                        "feat!: add a feature"
                    )),
                },
                Change {
                    change_type: ChangeType::Feature,
                    description: "add another feature".into(),
                    original_source: ChangeSource::ConventionalCommit(String::from(
                        "feat: add another feature"
                    )),
                }
            ]
        );
    }

    #[test]
    fn separate_breaking_messages() {
        let commits = [
            "fix: a bug\n\nBREAKING CHANGE: something broke",
            "feat: a features\n\nBREAKING CHANGE: something else broke",
        ];
        let changes =
            changes_from_commit_messages(&commits, None, &Sections::default()).collect_vec();
        assert_eq!(
            changes,
            vec![
                Change {
                    change_type: ChangeType::Breaking,
                    description: "something broke".into(),
                    original_source: ChangeSource::ConventionalCommit(String::from(
                        "fix: a bug\n\tContaining footer BREAKING CHANGE: something broke"
                    )),
                },
                Change {
                    change_type: ChangeType::Fix,
                    description: "a bug".into(),
                    original_source: ChangeSource::ConventionalCommit(String::from("fix: a bug")),
                },
                Change {
                    change_type: ChangeType::Breaking,
                    description: "something else broke".into(),
                    original_source: ChangeSource::ConventionalCommit(String::from(
                        "feat: a features\n\tContaining footer BREAKING CHANGE: something else broke"
                    )),
                },
                Change {
                    change_type: ChangeType::Feature,
                    description: "a features".into(),
                    original_source: ChangeSource::ConventionalCommit(String::from(
                        "feat: a features"
                    )),
                },
            ]
        );
    }

    #[test]
    fn scopes_used_but_none_defined() {
        let commits = [
            "feat(scope)!: Wrong scope breaking change!",
            "fix: No scope",
        ];
        let changes =
            changes_from_commit_messages(&commits, None, &Sections::default()).collect_vec();
        assert_eq!(
            changes,
            vec![
                Change {
                    change_type: ChangeType::Breaking,
                    description: "Wrong scope breaking change!".into(),
                    original_source: ChangeSource::ConventionalCommit(String::from(
                        "feat(scope)!: Wrong scope breaking change!"
                    )),
                },
                Change {
                    change_type: ChangeType::Fix,
                    description: "No scope".into(),
                    original_source: ChangeSource::ConventionalCommit(String::from(
                        "fix: No scope"
                    )),
                }
            ]
        );
    }

    #[test]
    fn filter_scopes() {
        let commits = [
            "feat(wrong_scope)!: Wrong scope breaking change!",
            "feat(scope): Scoped feature",
            "fix: No scope",
        ];

        let changes = changes_from_commit_messages(
            &commits,
            Some(&vec![String::from("scope")]),
            &Sections::default(),
        )
        .collect_vec();
        assert_eq!(
            changes,
            vec![
                Change {
                    change_type: ChangeType::Feature,
                    description: "Scoped feature".into(),
                    original_source: ChangeSource::ConventionalCommit(String::from(
                        "feat(scope): Scoped feature"
                    )),
                },
                Change {
                    change_type: ChangeType::Fix,
                    description: "No scope".into(),
                    original_source: ChangeSource::ConventionalCommit(String::from(
                        "fix: No scope"
                    )),
                },
            ]
        );
    }

    #[test]
    fn custom_footers() {
        let commits = ["chore: ignored type\n\nignored-footer: ignored\ncustom-footer: hello"];
        let changelog_sections = Sections(vec![(
            "custom section".into(),
            vec![ChangeType::Custom(SectionSource::CommitFooter(
                "custom-footer".into(),
            ))],
        )]);
        let changes =
            changes_from_commit_messages(&commits, None, &changelog_sections).collect_vec();
        assert_eq!(
            changes,
            vec![Change {
                change_type: ChangeType::Custom(SectionSource::CommitFooter(
                    "custom-footer".into()
                )),
                description: "hello".into(),
                original_source: ChangeSource::ConventionalCommit(String::from(
                    "chore: ignored type\n\tContaining footer custom-footer: hello"
                )),
            }]
        );
    }
}
