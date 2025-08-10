use std::{cmp::Ordering, fmt::Display, sync::Arc};

use changesets::PackageChange;
use git_conventional::FooterToken;
use itertools::Itertools;

use crate::release_notes::{CommitFooter, CustomChangeType, SectionSource};

pub mod conventional_commit;

pub const CHANGESET_DIR: &str = ".changeset";

/// Git commit information including hash and author.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitInfo {
    pub hash: String,
    pub author_name: String,
}

/// A change to one or more packages.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Change {
    pub change_type: ChangeType,
    pub summary: String,
    pub details: Option<String>,
    pub original_source: ChangeSource,
    pub git: Option<GitInfo>,
}

impl Change {
    /// Convert [`PackageChange`] into [`Change`], optionally including info from the commit that
    /// added the change files.
    pub(crate) fn from_changeset<'a>(
        changes: impl IntoIterator<Item = (&'a PackageChange, Option<GitInfo>)>,
    ) -> impl Iterator<Item = Self> {
        changes.into_iter().map(|(package_change, git_info)| {
            Self::from_package_change_and_commit(package_change, git_info)
        })
    }

    /// Create a single change from a package change with explicit commit information.
    fn from_package_change_and_commit(
        package_change: &PackageChange,
        git: Option<GitInfo>,
    ) -> Self {
        let mut lines = package_change
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
        let details: String = lines.skip_while(|it| it.is_empty()).join("\n");

        Self {
            change_type: ChangeType::from(&package_change.change_type),
            summary,
            details: (!details.is_empty()).then_some(details),
            original_source: ChangeSource::ChangeFile {
                id: package_change.unique_id.clone(),
            },
            git,
        }
    }
}

impl Ord for Change {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.details.is_some(), other.details.is_some()) {
            (false, true) => Ordering::Less,
            (true, false) => Ordering::Greater,
            _ => Ordering::Equal,
        }
    }
}

impl PartialOrd for Change {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum ChangeType {
    Breaking,
    Feature,
    Fix,
    Custom(SectionSource),
}

impl ChangeType {
    #[must_use]
    pub fn to_changeset_type(&self) -> Option<changesets::ChangeType> {
        match self {
            Self::Breaking => Some(changesets::ChangeType::Major),
            Self::Feature => Some(changesets::ChangeType::Minor),
            Self::Fix => Some(changesets::ChangeType::Patch),
            Self::Custom(SectionSource::CustomChangeType(custom)) => {
                Some(changesets::ChangeType::Custom(custom.to_string()))
            }
            Self::Custom(SectionSource::CommitFooter(_)) => None,
        }
    }
}

impl From<&ChangeType> for changesets::ChangeType {
    fn from(value: &ChangeType) -> Self {
        match value {
            ChangeType::Breaking => Self::Major,
            ChangeType::Feature => Self::Minor,
            ChangeType::Fix => Self::Patch,
            ChangeType::Custom(custom) => Self::Custom(custom.to_string()),
        }
    }
}

impl From<&changesets::ChangeType> for ChangeType {
    fn from(value: &changesets::ChangeType) -> Self {
        match value {
            changesets::ChangeType::Major => Self::Breaking,
            changesets::ChangeType::Minor => Self::Feature,
            changesets::ChangeType::Patch => Self::Fix,
            changesets::ChangeType::Custom(custom) => {
                Self::Custom(SectionSource::CustomChangeType(custom.clone().into()))
            }
        }
    }
}

impl From<CustomChangeType> for ChangeType {
    fn from(custom: CustomChangeType) -> Self {
        changesets::ChangeType::from(String::from(custom)).into()
    }
}

impl From<changesets::ChangeType> for ChangeType {
    fn from(change_type: changesets::ChangeType) -> Self {
        match change_type {
            changesets::ChangeType::Major => Self::Breaking,
            changesets::ChangeType::Minor => Self::Feature,
            changesets::ChangeType::Patch => Self::Fix,
            changesets::ChangeType::Custom(custom) => {
                Self::Custom(SectionSource::CustomChangeType(custom.into()))
            }
        }
    }
}

impl From<CommitFooter> for ChangeType {
    fn from(footer: CommitFooter) -> Self {
        Self::Custom(SectionSource::CommitFooter(footer))
    }
}

impl From<FooterToken<'_>> for ChangeType {
    fn from(footer: FooterToken) -> Self {
        if footer.breaking() {
            Self::Breaking
        } else {
            Self::Custom(SectionSource::CommitFooter(CommitFooter::from(footer)))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChangeSource {
    ConventionalCommit { description: String },
    ChangeFile { id: Arc<changesets::UniqueId> },
}

impl Display for ChangeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConventionalCommit {
                description: message,
                ..
            } => write!(f, "commit {message}"),
            Self::ChangeFile { id, .. } => write!(f, "changeset {}", id.to_file_name()),
        }
    }
}

#[cfg(test)]
mod test_parse_changes {
    use changesets::{PackageChange, UniqueId};
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::changes::{ChangeSource, ChangeType};

    #[test]
    fn simple_changeset() {
        let package_change = PackageChange {
            unique_id: Arc::new(UniqueId::exact("1234")),
            change_type: changesets::ChangeType::Minor,
            summary: "# a feature\n\n\n\n".into(),
        };
        let change = Change::from_package_change_and_commit(&package_change, None);
        assert_eq!(change.summary, "a feature");
        assert!(change.details.is_none());
        assert_eq!(
            change.original_source,
            ChangeSource::ChangeFile {
                id: package_change.unique_id,
            }
        );
        assert_eq!(change.change_type, ChangeType::Feature);
    }

    #[test]
    fn complex_changeset() {
        let package_change = PackageChange {
            unique_id: Arc::new(UniqueId::exact("1234")),
            change_type: changesets::ChangeType::Minor,
            summary: "# a feature\n\nwith details\n\n- first\n- second".into(),
        };
        let change = Change::from_package_change_and_commit(&package_change, None);
        assert_eq!(change.summary, "a feature");
        assert_eq!(change.details.unwrap(), "with details\n\n- first\n- second");
        assert_eq!(
            change.original_source,
            ChangeSource::ChangeFile {
                id: package_change.unique_id,
            }
        );
        assert_eq!(change.change_type, ChangeType::Feature);
    }
    #[test]
    fn from_package_changes_with_commits() {
        let changes_with_commits = [
            (
                &PackageChange {
                    unique_id: Arc::new(UniqueId::exact("committed-change")),
                    change_type: changesets::ChangeType::Major,
                    summary: "# Breaking change".into(),
                },
                Some(GitInfo {
                    author_name: "Bob".to_string(),
                    hash: "def456".to_string(),
                }),
            ),
            (
                &PackageChange {
                    unique_id: Arc::new(UniqueId::exact("uncommitted-change")),
                    change_type: changesets::ChangeType::Minor,
                    summary: "# Feature without commit".into(),
                },
                None,
            ),
        ];

        let changes: Vec<Change> = Change::from_changeset(changes_with_commits).collect();

        assert_eq!(changes.len(), 2);

        // First change has commit info
        assert_eq!(changes[0].summary, "Breaking change");
        assert_eq!(changes[0].git.as_ref().unwrap().author_name, "Bob");
        assert_eq!(changes[0].git.as_ref().unwrap().hash, "def456");

        // Second change has no commit info
        assert_eq!(changes[1].summary, "Feature without commit");
        assert_eq!(changes[1].git, None);
    }
}
