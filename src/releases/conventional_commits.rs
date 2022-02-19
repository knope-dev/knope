use color_eyre::eyre::{Result, WrapErr};
use git_conventional::{Commit, Type};

use crate::git::get_commit_messages_after_last_tag;
use crate::releases::changelog::new_changelog_lines;
use crate::semver::{bump_version, get_version, ConventionalRule, Rule};
use crate::state::{Initial, IssueSelected, ReleasePrepared, State};

use super::changelog::add_version_to_changelog;

#[derive(Debug)]
struct ConventionalCommits {
    rule: ConventionalRule,
    features: Vec<String>,
    fixes: Vec<String>,
    breaking_changes: Vec<String>,
}

impl ConventionalCommits {
    fn from_commits(commits: Vec<Commit>) -> Self {
        let mut rule = ConventionalRule::Patch;
        let mut features = Vec::new();
        let mut fixes = Vec::new();
        let mut breaking_changes = Vec::new();

        for commit in commits {
            if let Some(breaking_message) = commit.breaking_description() {
                rule = ConventionalRule::Major;
                breaking_changes.push(breaking_message.to_string());
                if breaking_message == commit.description() {
                    // There is no separate breaking change message, so the normal description is used.
                    // Don't include the same message elsewhere.
                    continue;
                }
            }

            if commit.type_() == Type::FEAT {
                features.push(commit.description().to_string());
                if !matches!(rule, ConventionalRule::Major) {
                    rule = ConventionalRule::Minor;
                }
            } else if commit.type_() == Type::FIX {
                fixes.push(commit.description().to_string());
            }
        }

        ConventionalCommits {
            rule,
            features,
            fixes,
            breaking_changes,
        }
    }
}

#[cfg(test)]
mod test_conventional_commits {
    use super::*;

    #[test]
    fn non_breaking_features() {
        let commits = vec![
            Commit::parse("feat: add a feature").unwrap(),
            Commit::parse("feat: another feature").unwrap(),
        ];
        let conventional_commits = ConventionalCommits::from_commits(commits);
        assert_eq!(conventional_commits.rule, ConventionalRule::Minor);
        assert_eq!(
            conventional_commits.features,
            vec![
                String::from("add a feature"),
                String::from("another feature")
            ]
        );
        assert_eq!(conventional_commits.fixes, Vec::<String>::new());
        assert_eq!(conventional_commits.breaking_changes, Vec::<String>::new());
    }

    #[test]
    fn non_breaking_fixes() {
        let commits = vec![
            Commit::parse("fix: a bug").unwrap(),
            Commit::parse("fix: another bug").unwrap(),
        ];
        let conventional_commits = ConventionalCommits::from_commits(commits);
        assert_eq!(conventional_commits.rule, ConventionalRule::Patch);
        assert_eq!(
            conventional_commits.fixes,
            vec![String::from("a bug"), String::from("another bug")]
        );
        assert_eq!(conventional_commits.features, Vec::<String>::new());
        assert_eq!(conventional_commits.breaking_changes, Vec::<String>::new());
    }

    #[test]
    fn mixed_fixes_and_features() {
        let commits = vec![
            Commit::parse("fix: a bug").unwrap(),
            Commit::parse("feat: add a feature").unwrap(),
        ];
        let conventional_commits = ConventionalCommits::from_commits(commits);
        assert_eq!(conventional_commits.rule, ConventionalRule::Minor);
        assert_eq!(conventional_commits.fixes, vec![String::from("a bug")]);
        assert_eq!(
            conventional_commits.features,
            vec![String::from("add a feature")]
        );
        assert_eq!(conventional_commits.breaking_changes, Vec::<String>::new());
    }

    #[test]
    fn breaking_feature() {
        let commits = vec![
            Commit::parse("fix: a bug").unwrap(),
            Commit::parse("feat!: add a feature").unwrap(),
            Commit::parse("feat: add another feature").unwrap(),
        ];
        let conventional_commits = ConventionalCommits::from_commits(commits);
        assert_eq!(conventional_commits.rule, ConventionalRule::Major);
        assert_eq!(conventional_commits.fixes, vec![String::from("a bug")]);
        assert_eq!(
            conventional_commits.features,
            vec![String::from("add another feature")]
        );
        assert_eq!(
            conventional_commits.breaking_changes,
            vec![String::from("add a feature")]
        );
    }

    #[test]
    fn breaking_fix() {
        let commits = vec![
            Commit::parse("fix!: a bug").unwrap(),
            Commit::parse("fix: another bug").unwrap(),
            Commit::parse("feat: add a feature").unwrap(),
        ];
        let conventional_commits = ConventionalCommits::from_commits(commits);
        assert_eq!(conventional_commits.rule, ConventionalRule::Major);
        assert_eq!(
            conventional_commits.fixes,
            vec![String::from("another bug")]
        );
        assert_eq!(
            conventional_commits.features,
            vec![String::from("add a feature")]
        );
        assert_eq!(
            conventional_commits.breaking_changes,
            vec![String::from("a bug")]
        );
    }

    #[test]
    fn fix_with_separate_breaking_message() {
        let commits = vec![
            Commit::parse("fix: a bug\n\nBREAKING CHANGE: something broke").unwrap(),
            Commit::parse("fix: another bug").unwrap(),
            Commit::parse("feat: add a feature").unwrap(),
        ];
        let conventional_commits = ConventionalCommits::from_commits(commits);
        assert_eq!(conventional_commits.rule, ConventionalRule::Major);
        assert_eq!(
            conventional_commits.fixes,
            vec![String::from("a bug"), String::from("another bug")]
        );
        assert_eq!(
            conventional_commits.features,
            vec![String::from("add a feature")]
        );
        assert_eq!(
            conventional_commits.breaking_changes,
            vec![String::from("something broke")]
        );
    }

    #[test]
    fn feature_with_separate_breaking_message() {
        let commits = vec![
            Commit::parse("feat: add a feature\n\nBREAKING CHANGE: something broke").unwrap(),
            Commit::parse("fix: a bug").unwrap(),
            Commit::parse("feat: add another feature").unwrap(),
        ];
        let conventional_commits = ConventionalCommits::from_commits(commits);
        assert_eq!(conventional_commits.rule, ConventionalRule::Major);
        assert_eq!(conventional_commits.fixes, vec![String::from("a bug")]);
        assert_eq!(
            conventional_commits.features,
            vec![
                String::from("add a feature"),
                String::from("add another feature")
            ]
        );
        assert_eq!(
            conventional_commits.breaking_changes,
            vec![String::from("something broke")]
        );
    }
}

fn get_conventional_commits_after_last_tag() -> Result<ConventionalCommits> {
    let commit_messages = get_commit_messages_after_last_tag()
        .wrap_err("Could not get commit messages after last tag.")?;
    let commits = commit_messages
        .iter()
        .filter_map(|message| Commit::parse(message.trim()).ok())
        .collect();
    Ok(ConventionalCommits::from_commits(commits))
}

pub(crate) fn update_project_from_conventional_commits(
    state: crate::State,
    changelog_path: &str,
    prerelease_label: Option<String>,
) -> Result<crate::State> {
    let ConventionalCommits {
        rule,
        features,
        fixes,
        breaking_changes,
    } = get_conventional_commits_after_last_tag()?;
    let rule = if let Some(prefix) = prerelease_label {
        Rule::Pre {
            label: prefix,
            fallback_rule: rule,
        }
    } else {
        Rule::from(rule)
    };
    let state = bump_version(state, rule).wrap_err("While bumping version")?;
    let new_version = get_version().wrap_err("While getting new version")?;
    let changelog_text =
        std::fs::read_to_string(changelog_path).wrap_err("While reading CHANGELOG.md")?;
    let new_version_string = new_version.to_string();
    let new_changes =
        new_changelog_lines(&new_version_string, &fixes, &features, &breaking_changes);
    let changelog = add_version_to_changelog(&changelog_text, &new_changes);
    std::fs::write(changelog_path, changelog)
        .wrap_err_with(|| format!("While writing to {}", changelog_path))?;
    match state {
        State::Initial(Initial {
            jira_config,
            github_state,
            github_config,
        })
        | State::IssueSelected(IssueSelected {
            jira_config,
            github_state,
            github_config,
            ..
        })
        | State::ReleasePrepared(ReleasePrepared {
            jira_config,
            github_state,
            github_config,
            ..
        }) => Ok(State::ReleasePrepared(ReleasePrepared {
            jira_config,
            github_state,
            github_config,
            release_notes: new_changes.join("\n"),
            new_version: new_version_string,
            is_prerelease: !new_version.version.pre.is_empty(),
        })),
    }
}
