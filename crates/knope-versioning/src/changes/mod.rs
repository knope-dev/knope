use std::{fmt::Display, sync::Arc};

use git_conventional::FooterToken;

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
    pub description: Arc<str>,
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
            .flat_map(|release_changes| release_changes.changes.clone().into_iter().map(Self::from))
    }
}

impl From<changesets::PackageChange> for Change {
    fn from(package_change: changesets::PackageChange) -> Self {
        Self {
            change_type: package_change.change_type.into(),
            description: package_change.summary,
            original_source: ChangeSource::ChangeFile(package_change.unique_id),
        }
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

impl From<ChangeType> for changesets::ChangeType {
    fn from(value: ChangeType) -> Self {
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
    ConventionalCommit(String),
    ChangeFile(Arc<changesets::UniqueId>),
}

impl Display for ChangeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConventionalCommit(commit) => write!(f, "commit {commit}"),
            Self::ChangeFile(id) => write!(f, "changeset {}", id.to_file_name()),
        }
    }
}
