use git_conventional::{Commit as ConventionalCommit, Footer, Type};
use tracing::debug;

use super::{Change, ChangeSource, ChangeType, GitInfo};
use crate::release_notes::Sections;

#[derive(Clone, Debug, Default)]
pub struct Commit {
    pub message: String,
    pub info: Option<GitInfo>,
}

/// Try to parse each commit message as a [conventional commit](https://www.conventionalcommits.org/).
///
/// # Filtering
///
/// 1. If the commit message doesn't follow the conventional commit format, it is ignored.
/// 2. For non-standard change types, only those included will be considered.
/// 3. For non-standard footers, only those included will be considered.
pub(crate) fn changes_from_commit_messages<'a>(
    commit_messages: &'a [Commit],
    scopes: Option<&'a Vec<String>>,
    changelog_sections: &'a Sections,
) -> impl Iterator<Item = Change> + 'a {
    if let Some(scopes) = scopes {
        debug!("Only checking commits with scopes: {scopes:?}");
    }
    commit_messages.iter().flat_map(move |commit| {
        changes_from_commit_message(commit, scopes, changelog_sections).into_iter()
    })
}

fn changes_from_commit_message(
    commit_info: &Commit,
    scopes: Option<&Vec<String>>,
    changelog_sections: &Sections,
) -> Vec<Change> {
    let Some(commit) = ConventionalCommit::parse(commit_info.message.trim()).ok() else {
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
            summary: footer.value().into(),
            details: None,
            original_source: ChangeSource::ConventionalCommit {
                description: format_commit_footer(&commit_summary, footer),
            },
            git: commit_info.info.clone(),
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
        summary: commit.description().into(),
        details: None,
        original_source: ChangeSource::ConventionalCommit {
            description: commit_summary,
        },
        git: commit_info.info.clone(),
    });

    changes
}

fn format_commit_summary(commit: &ConventionalCommit) -> String {
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
            Commit {
                message: "fix: a bug".to_string(),
                ..Default::default()
            },
            Commit {
                message: "fix!: a breaking bug fix".to_string(),
                ..Default::default()
            },
            Commit {
                message: "feat!: add a feature".to_string(),
                ..Default::default()
            },
            Commit {
                message: "feat: add another feature".to_string(),
                ..Default::default()
            },
        ];
        let changes =
            changes_from_commit_messages(commits, None, &Sections::default()).collect_vec();
        assert_eq!(
            changes,
            vec![
                Change {
                    change_type: ChangeType::Fix,
                    summary: "a bug".into(),
                    details: None,
                    original_source: ChangeSource::ConventionalCommit {
                        description: String::from("fix: a bug"),
                    },
                    git: None,
                },
                Change {
                    change_type: ChangeType::Breaking,
                    summary: "a breaking bug fix".into(),
                    details: None,
                    original_source: ChangeSource::ConventionalCommit {
                        description: String::from("fix!: a breaking bug fix"),
                    },
                    git: None,
                },
                Change {
                    change_type: ChangeType::Breaking,
                    summary: "add a feature".into(),
                    details: None,
                    original_source: ChangeSource::ConventionalCommit {
                        description: String::from("feat!: add a feature"),
                    },
                    git: None,
                },
                Change {
                    change_type: ChangeType::Feature,
                    summary: "add another feature".into(),
                    details: None,
                    original_source: ChangeSource::ConventionalCommit {
                        description: String::from("feat: add another feature"),
                    },
                    git: None,
                }
            ]
        );
    }

    #[test]
    fn separate_breaking_messages() {
        let commits = [
            Commit {
                message: "fix: a bug\n\nBREAKING CHANGE: something broke".to_string(),
                ..Default::default()
            },
            Commit {
                message: "feat: a features\n\nBREAKING CHANGE: something else broke".to_string(),
                ..Default::default()
            },
        ];
        let changes =
            changes_from_commit_messages(&commits, None, &Sections::default()).collect_vec();
        assert_eq!(
            changes,
            vec![
                Change {
                    change_type: ChangeType::Breaking,
                    summary: "something broke".into(),
                    details: None,
                    original_source: ChangeSource::ConventionalCommit {
                        description: String::from(
                            "fix: a bug\n\tContaining footer BREAKING CHANGE: something broke"
                        ),
                    },
                    git: None,
                },
                Change {
                    change_type: ChangeType::Fix,
                    summary: "a bug".into(),
                    details: None,
                    original_source: ChangeSource::ConventionalCommit {
                        description: String::from("fix: a bug"),
                    },
                    git: None,
                },
                Change {
                    change_type: ChangeType::Breaking,
                    summary: "something else broke".into(),
                    details: None,
                    original_source: ChangeSource::ConventionalCommit {
                        description: String::from(
                            "feat: a features\n\tContaining footer BREAKING CHANGE: something else broke"
                        ),
                    },
                    git: None,
                },
                Change {
                    change_type: ChangeType::Feature,
                    summary: "a features".into(),
                    details: None,
                    original_source: ChangeSource::ConventionalCommit {
                        description: String::from("feat: a features"),
                    },
                    git: None,
                },
            ]
        );
    }

    #[test]
    fn scopes_used_but_none_defined() {
        let commits = [
            Commit {
                message: "feat(scope)!: Wrong scope breaking change!".to_string(),
                ..Default::default()
            },
            Commit {
                message: "fix: No scope".to_string(),
                ..Default::default()
            },
        ];
        let changes =
            changes_from_commit_messages(&commits, None, &Sections::default()).collect_vec();
        assert_eq!(
            changes,
            vec![
                Change {
                    change_type: ChangeType::Breaking,
                    summary: "Wrong scope breaking change!".into(),
                    details: None,
                    original_source: ChangeSource::ConventionalCommit {
                        description: String::from("feat(scope)!: Wrong scope breaking change!"),
                    },
                    git: None,
                },
                Change {
                    change_type: ChangeType::Fix,
                    summary: "No scope".into(),
                    details: None,
                    original_source: ChangeSource::ConventionalCommit {
                        description: String::from("fix: No scope"),
                    },
                    git: None,
                }
            ]
        );
    }

    #[test]
    fn filter_scopes() {
        let commits = [
            Commit {
                message: "feat(wrong_scope)!: Wrong scope breaking change!".to_string(),
                ..Default::default()
            },
            Commit {
                message: "feat(scope): Scoped feature".to_string(),
                ..Default::default()
            },
            Commit {
                message: "fix: No scope".to_string(),
                ..Default::default()
            },
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
                    summary: "Scoped feature".into(),
                    details: None,
                    original_source: ChangeSource::ConventionalCommit {
                        description: String::from("feat(scope): Scoped feature"),
                    },
                    git: None,
                },
                Change {
                    change_type: ChangeType::Fix,
                    summary: "No scope".into(),
                    details: None,
                    original_source: ChangeSource::ConventionalCommit {
                        description: String::from("fix: No scope"),
                    },
                    git: None,
                },
            ]
        );
    }

    #[test]
    fn custom_footers() {
        let commits = [Commit {
            message: "chore: ignored type\n\nignored-footer: ignored\ncustom-footer: hello"
                .to_string(),
            ..Default::default()
        }];
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
                summary: "hello".into(),
                details: None,
                original_source: ChangeSource::ConventionalCommit {
                    description: String::from(
                        "chore: ignored type\n\tContaining footer custom-footer: hello"
                    ),
                },
                git: None,
            }]
        );
    }
}
