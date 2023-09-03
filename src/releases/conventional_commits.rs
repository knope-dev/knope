use git_conventional::{Commit, Type};
use log::debug;

use crate::{
    config::CommitFooter,
    git::get_commit_messages_after_last_stable_version,
    releases::{package::ChangelogSectionSource, Change, ChangeType, Package},
    step::StepError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ConventionalCommit {
    pub(crate) change_type: ChangeType,
    pub(crate) message: String,
}

impl ConventionalCommit {
    fn from_commit_messages(
        commit_messages: &[String],
        consider_scopes: bool,
        package: &Package,
    ) -> Vec<Self> {
        let commits = commit_messages
            .iter()
            .filter_map(|message| Commit::parse(message.trim()).ok())
            .filter(|commit| {
                if !consider_scopes {
                    return true;
                }
                match (commit.scope(), &package.scopes) {
                    (None, _) => true,
                    (Some(_), None) => false,
                    (Some(scope), Some(scopes)) => scopes.contains(&scope.to_string()),
                }
            })
            .collect();
        debug!("Selected commits: {:?}", commits);
        Self::from_commits(package, commits)
    }

    fn from_commits(package: &Package, commits: Vec<Commit>) -> Vec<Self> {
        let mut conventional_commits = Vec::with_capacity(commits.len());

        for commit in commits {
            for footer in commit.footers() {
                let source: ChangelogSectionSource = CommitFooter::from(footer.token()).into();
                if package.extra_changelog_sections.contains_key(&source) {
                    conventional_commits.push(Self {
                        change_type: ChangeType::from(source),
                        message: footer.value().to_string(),
                    });
                }
            }
            if let Some(breaking_message) = commit.breaking_description() {
                conventional_commits.push(Self {
                    change_type: ChangeType::Breaking,
                    message: breaking_message.to_string(),
                });
                if breaking_message == commit.description() {
                    // There is no separate breaking change message, so the normal description is used.
                    // Don't include the same message elsewhere.
                    continue;
                }
            }

            if commit.type_() == Type::FEAT {
                conventional_commits.push(Self {
                    change_type: ChangeType::Feature,
                    message: commit.description().to_string(),
                });
            } else if commit.type_() == Type::FIX {
                conventional_commits.push(Self {
                    change_type: ChangeType::Fix,
                    message: commit.description().to_string(),
                });
            }
        }
        conventional_commits
    }
}

#[cfg(test)]
mod test_conventional_commits {
    use indexmap::IndexMap;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::releases::package::ChangelogSectionSource;

    #[test]
    fn commit_types() {
        let commits = vec![
            Commit::parse("fix: a bug").unwrap(),
            Commit::parse("fix!: a breaking bug fix").unwrap(),
            Commit::parse("feat!: add a feature").unwrap(),
            Commit::parse("feat: add another feature").unwrap(),
        ];
        let package = Package::default();
        let conventional_commits = ConventionalCommit::from_commits(&package, commits);
        assert_eq!(
            conventional_commits,
            vec![
                ConventionalCommit {
                    change_type: ChangeType::Fix,
                    message: String::from("a bug")
                },
                ConventionalCommit {
                    change_type: ChangeType::Breaking,
                    message: String::from("a breaking bug fix")
                },
                ConventionalCommit {
                    change_type: ChangeType::Breaking,
                    message: String::from("add a feature")
                },
                ConventionalCommit {
                    change_type: ChangeType::Feature,
                    message: String::from("add another feature")
                }
            ]
        );
    }

    #[test]
    fn separate_breaking_messages() {
        let commits = vec![
            Commit::parse("fix: a bug\n\nBREAKING CHANGE: something broke").unwrap(),
            Commit::parse("feat: a features\n\nBREAKING CHANGE: something else broke").unwrap(),
        ];
        let package = Package::default();
        let conventional_commits = ConventionalCommit::from_commits(&package, commits);
        assert_eq!(
            conventional_commits,
            vec![
                ConventionalCommit {
                    change_type: ChangeType::Breaking,
                    message: String::from("something broke")
                },
                ConventionalCommit {
                    change_type: ChangeType::Fix,
                    message: String::from("a bug")
                },
                ConventionalCommit {
                    change_type: ChangeType::Breaking,
                    message: String::from("something else broke")
                },
                ConventionalCommit {
                    change_type: ChangeType::Feature,
                    message: String::from("a features")
                },
            ]
        );
    }

    #[test]
    fn no_commits() {
        let commits = Vec::<Commit>::new();
        let package = Package::default();
        let conventional_commits = ConventionalCommit::from_commits(&package, commits);
        assert_eq!(conventional_commits, Vec::<ConventionalCommit>::new());
    }

    #[test]
    fn dont_consider_scopes() {
        let commits = [
            "feat(wrong_scope)!: Wrong scope breaking change!",
            "fix: No scope",
        ]
        .map(String::from);
        let conventional_commits = ConventionalCommit::from_commit_messages(
            &commits,
            false,
            &Package {
                scopes: Some(vec![String::from("scope")]),
                ..Package::default()
            },
        );
        assert_eq!(
            conventional_commits,
            vec![
                ConventionalCommit {
                    change_type: ChangeType::Breaking,
                    message: String::from("Wrong scope breaking change!")
                },
                ConventionalCommit {
                    change_type: ChangeType::Fix,
                    message: String::from("No scope")
                },
            ]
        );
    }

    #[test]
    fn consider_scopes_but_none_defined() {
        let commits = [
            "feat(scope)!: Wrong scope breaking change!",
            "fix: No scope",
        ]
        .map(String::from);
        let conventional_commits =
            ConventionalCommit::from_commit_messages(&commits, true, &Package::default());
        assert_eq!(
            conventional_commits,
            vec![ConventionalCommit {
                change_type: ChangeType::Fix,
                message: String::from("No scope")
            },]
        );
    }

    #[test]
    fn consider_scopes() {
        let commits = [
            "feat(wrong_scope)!: Wrong scope breaking change!",
            "feat(scope): Right scope feature",
            "fix: No scope",
        ]
        .map(String::from);
        let conventional_commits = ConventionalCommit::from_commit_messages(
            &commits,
            true,
            &Package {
                scopes: Some(vec![String::from("scope")]),
                ..Package::default()
            },
        );
        assert_eq!(
            conventional_commits,
            vec![
                ConventionalCommit {
                    change_type: ChangeType::Feature,
                    message: String::from("Right scope feature")
                },
                ConventionalCommit {
                    change_type: ChangeType::Fix,
                    message: String::from("No scope")
                },
            ]
        );
    }

    #[test]
    fn custom_footers() {
        let commits = [String::from(
            "chore: ignored type\n\nignored-footer: ignored\ncustom-footer: hello",
        )];
        let mut extra_changelog_sections = IndexMap::new();
        extra_changelog_sections.insert(
            CommitFooter::from("custom-footer").into(),
            "custom section".into(),
        );
        let conventional_commits = ConventionalCommit::from_commit_messages(
            &commits,
            false,
            &Package {
                extra_changelog_sections,
                ..Package::default()
            },
        );
        assert_eq!(
            conventional_commits,
            vec![ConventionalCommit {
                change_type: ChangeType::Custom(ChangelogSectionSource::CommitFooter(
                    "custom-footer".into()
                )),
                message: String::from("hello")
            },]
        );
    }
}

fn get_conventional_commits_after_last_stable_version(
    package: &Package,
    consider_scopes: bool,
) -> Result<Vec<ConventionalCommit>, StepError> {
    let commit_messages = get_commit_messages_after_last_stable_version(package.name.as_ref())?;
    Ok(ConventionalCommit::from_commit_messages(
        &commit_messages,
        consider_scopes,
        package,
    ))
}

pub(crate) fn add_releases_from_conventional_commits(
    packages: Vec<Package>,
) -> Result<Vec<Package>, StepError> {
    let consider_scopes = packages.iter().any(|package| package.scopes.is_some());
    packages
        .into_iter()
        .map(|package| add_release_for_package(package, consider_scopes))
        .collect()
}

fn add_release_for_package(
    mut package: Package,
    consider_scopes: bool,
) -> Result<Package, StepError> {
    get_conventional_commits_after_last_stable_version(&package, consider_scopes).map(|commits| {
        if commits.is_empty() {
            package
        } else {
            package.pending_changes = commits
                .into_iter()
                .map(Change::ConventionalCommit)
                .collect();
            package
        }
    })
}
