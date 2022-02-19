use color_eyre::eyre::{Result, WrapErr};
use git_conventional::{Commit, Type};

use crate::changelog::{add_version_to_changelog, Version};
use crate::git::get_commit_messages_after_last_tag;
use crate::semver::{bump_version, get_version, Rule};

#[derive(Debug)]
struct ConventionalCommits {
    rule: Rule,
    features: Vec<String>,
    fixes: Vec<String>,
    breaking_changes: Vec<String>,
}

impl ConventionalCommits {
    fn from_commits(commits: Vec<Commit>) -> Self {
        let mut rule = Rule::Patch;
        let mut features = Vec::new();
        let mut fixes = Vec::new();
        let mut breaking_changes = Vec::new();

        for commit in commits {
            if let Some(breaking_message) = commit.breaking_description() {
                rule = Rule::Major;
                breaking_changes.push(breaking_message.to_string());
                if breaking_message == commit.description() {
                    // There is no separate breaking change message, so the normal description is used.
                    // Don't include the same message elsewhere.
                    continue;
                }
            }

            if commit.type_() == Type::FEAT {
                features.push(commit.description().to_string());
                if !matches!(rule, Rule::Major) {
                    rule = Rule::Minor;
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
        assert_eq!(conventional_commits.rule, Rule::Minor);
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
        assert_eq!(conventional_commits.rule, Rule::Patch);
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
        assert_eq!(conventional_commits.rule, Rule::Minor);
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
        assert_eq!(conventional_commits.rule, Rule::Major);
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
        assert_eq!(conventional_commits.rule, Rule::Major);
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
        assert_eq!(conventional_commits.rule, Rule::Major);
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
        assert_eq!(conventional_commits.rule, Rule::Major);
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
        mut rule,
        features,
        fixes,
        breaking_changes,
    } = get_conventional_commits_after_last_tag()?;
    if let Some(prerelease_label) = prerelease_label {
        rule = Rule::Pre(prerelease_label);
    };
    let state = bump_version(state, &rule).wrap_err("While bumping version")?;
    let new_version = get_version().wrap_err("While getting new version")?;
    let changelog_text =
        std::fs::read_to_string(changelog_path).wrap_err("While reading CHANGELOG.md")?;
    let changelog_version = Version {
        title: new_version.to_string(),
        fixes,
        features,
        breaking_changes,
    };
    let changelog = add_version_to_changelog(&changelog_text, &changelog_version);
    std::fs::write(changelog_path, changelog)
        .wrap_err_with(|| format!("While writing to {}", changelog_path))?;
    Ok(state)
}
