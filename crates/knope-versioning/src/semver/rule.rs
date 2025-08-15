use std::fmt::Display;

use tracing::debug;

use super::Label;
use crate::changes::{Change, ChangeType};

/// The various rules that can be used when bumping semantic versions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rule {
    Stable(Stable),
    Pre { label: Label, stable_rule: Stable },
    Release,
}

impl From<Stable> for Rule {
    fn from(conventional_rule: Stable) -> Self {
        Self::Stable(conventional_rule)
    }
}

/// The rules that only apply to stable versions (no pre-releases)
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Stable {
    Major,
    Minor,
    #[default]
    Patch,
}

impl<'a, T: IntoIterator<Item = &'a Change>> From<T> for Stable {
    fn from(changes: T) -> Self {
        changes
            .into_iter()
            .map(|change| {
                let rule = Self::from(&change.change_type);
                debug!(
                    "{change_source}\n\timplies rule {rule}",
                    change_source = change.original_source
                );
                rule
            })
            .max()
            .unwrap_or_default()
    }
}

impl Display for Stable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Stable::Major => write!(f, "MAJOR"),
            Stable::Minor => write!(f, "MINOR"),
            Stable::Patch => write!(f, "PATCH"),
        }
    }
}

impl Ord for Stable {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Major, Self::Major)
            | (Self::Minor, Self::Minor)
            | (Self::Patch, Self::Patch) => std::cmp::Ordering::Equal,
            (Self::Major, _) | (_, Self::Patch) => std::cmp::Ordering::Greater,
            (_, Self::Major) | (Self::Patch, _) => std::cmp::Ordering::Less,
        }
    }
}

impl PartialOrd for Stable {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl From<&ChangeType> for Stable {
    fn from(value: &ChangeType) -> Self {
        match value {
            ChangeType::Feature => Self::Minor,
            ChangeType::Breaking => Self::Major,
            ChangeType::Custom(_) | ChangeType::Fix => Self::Patch,
        }
    }
}
