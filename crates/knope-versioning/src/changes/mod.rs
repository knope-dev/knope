use std::{cmp::Ordering, fmt::Display, sync::Arc};

use git_conventional::FooterToken;
use itertools::Itertools;

use crate::{
    package,
    release_notes::{CommitFooter, CustomChangeType, SectionSource},
};

pub mod conventional_commit;

pub const CHANGESET_DIR: &str = ".changeset";

/// A change to one or more packages.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Change {
    pub change_type: ChangeType,
    pub summary: String,
    pub details: Option<String>,
    pub original_source: ChangeSource,
}

impl Change {
    pub fn from_changesets<'a>(
        package_name: &'a package::Name,
        releases: &'a [changesets::Release],
    ) -> impl Iterator<Item = Self> + 'a {
        releases
            .iter()
            .find(|release| *package_name == release.package_name)
            .into_iter()
            .flat_map(|release_changes| release_changes.changes.iter().map(Self::from))
    }
}

impl From<&changesets::PackageChange> for Change {
    fn from(package_change: &changesets::PackageChange) -> Self {
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
            original_source: ChangeSource::ChangeFile(package_change.unique_id.clone()),
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
    ConventionalCommit {
        description: String,
        author_name: Option<String>,
        hash: Option<String>,
    },
    ChangeFile(Arc<changesets::UniqueId>),
}

impl Display for ChangeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConventionalCommit {
                description: message,
                ..
            } => write!(f, "commit {message}"),
            Self::ChangeFile(id) => write!(f, "changeset {}", id.to_file_name()),
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
        let change = Change::from(&package_change);
        assert_eq!(change.summary, "a feature");
        assert!(change.details.is_none());
        assert_eq!(
            change.original_source,
            ChangeSource::ChangeFile(package_change.unique_id)
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
        let change = Change::from(&package_change);
        assert_eq!(change.summary, "a feature");
        assert_eq!(change.details.unwrap(), "with details\n\n- first\n- second");
        assert_eq!(
            change.original_source,
            ChangeSource::ChangeFile(package_change.unique_id)
        );
        assert_eq!(change.change_type, ChangeType::Feature);
    }
}
