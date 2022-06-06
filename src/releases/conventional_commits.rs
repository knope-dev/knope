use git_conventional::{Commit, Type};

use crate::git::get_commit_messages_after_last_tag;
use crate::step::StepError;
use crate::{state, step, RunType};

use super::changelog::{add_version_to_changelog, new_changelog_lines};
use super::semver::{bump_version, ConventionalRule, Rule};
use super::Release;

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

fn get_conventional_commits_after_last_tag() -> Result<ConventionalCommits, StepError> {
    let commit_messages = get_commit_messages_after_last_tag()?;
    let commits = commit_messages
        .iter()
        .filter_map(|message| Commit::parse(message.trim()).ok())
        .collect();
    Ok(ConventionalCommits::from_commits(commits))
}

pub(crate) fn update_project_from_conventional_commits(
    run_type: RunType,
    prepare_release: step::PrepareRelease,
) -> Result<RunType, StepError> {
    let ConventionalCommits {
        rule,
        features,
        fixes,
        breaking_changes,
    } = get_conventional_commits_after_last_tag()?;
    let step::PrepareRelease { prerelease_label } = prepare_release;

    let (mut state, dry_run_stdout) = match run_type {
        RunType::DryRun { state, stdout } => (state, Some(stdout)),
        RunType::Real(state) => (state, None),
    };

    let changelog_path = if state.packages.is_empty() {
        return Err(StepError::no_defined_packages_with_help());
    } else if state.packages.len() > 1 {
        return Err(StepError::TooManyPackages);
    } else {
        state.packages.first().unwrap().changelog.as_ref()
    };

    let rule = if let Some(prefix) = prerelease_label {
        Rule::Pre {
            label: prefix,
            fallback_rule: rule,
        }
    } else {
        Rule::from(rule)
    };
    let new_version = bump_version(rule, dry_run_stdout.is_some(), &state.packages)?;
    let new_version_string = new_version.to_string();
    let new_changes =
        new_changelog_lines(&new_version_string, &fixes, &features, &breaking_changes);

    state.release = state::Release::Prepared(Release {
        version: new_version,
        changelog: new_changes.join("\n"),
    });

    if let Some(mut stdout) = dry_run_stdout {
        writeln!(stdout, "Would bump version to {}", &new_version_string)?;
        if let Some(changelog_path) = changelog_path {
            writeln!(
                stdout,
                "Would add the following to {}: \n{}",
                &changelog_path,
                new_changes.join("\n")
            )?;
        }
        Ok(RunType::DryRun { state, stdout })
    } else {
        if let Some(changelog_path) = changelog_path {
            let changelog_text = std::fs::read_to_string(&changelog_path)?;
            let changelog = add_version_to_changelog(&changelog_text, &new_changes);
            std::fs::write(&changelog_path, changelog)?;
        }
        Ok(RunType::Real(state))
    }
}
