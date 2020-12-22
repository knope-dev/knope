use color_eyre::eyre::{Result, WrapErr};
use git_conventional::{Commit, FEAT, FIX};

use crate::changelog::{Changelog, Version};
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
            if commit.breaking() {
                rule = Rule::Major;
                breaking_changes.push(commit.description().to_string());
            } else if commit.type_() == FEAT {
                features.push(commit.description().to_string());
                if !matches!(rule, Rule::Major) {
                    rule = Rule::Minor;
                }
            } else if commit.type_() == FIX {
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

fn get_conventional_commits_after_last_tag() -> Result<ConventionalCommits> {
    let commit_messages = get_commit_messages_after_last_tag()
        .wrap_err("Could not get commit messages after last tag.")?;
    let commits = commit_messages
        .iter()
        .filter_map(|message| Commit::parse(message).ok())
        .collect();
    Ok(ConventionalCommits::from_commits(commits))
}

pub(crate) fn update_project_from_conventional_commits(
    state: crate::State,
) -> Result<crate::State> {
    let ConventionalCommits {
        rule,
        features,
        fixes,
        breaking_changes,
    } = get_conventional_commits_after_last_tag()?;
    let state = bump_version(state, &rule).wrap_err("While bumping version")?;
    let new_version = get_version().wrap_err("While getting new version")?;
    let changelog_text =
        std::fs::read_to_string("CHANGELOG.md").wrap_err("While reading CHANGELOG.md")?;
    let changelog = Changelog::from_markdown(&changelog_text);
    let changelog_version = Version {
        title: new_version.to_string(),
        fixes,
        features,
        breaking_changes,
    };
    let changelog = changelog.add_version(changelog_version);
    std::fs::write("CHANGELOG.md", changelog.into_markdown())
        .wrap_err("While writing to CHANGELOG.md")?;
    Ok(state)
}
