mod changelog;

use std::{fmt, fmt::Display};

pub use changelog::*;
use git_conventional::FooterToken;
use serde::{Deserialize, Serialize};

use crate::changes::ChangeType;

/// Defines how release notes are handled for a package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseNotes {
    pub sections: Sections,
}

/// Where a custom release section comes from, for example, the custom change type "extra" in
/// a change file might correspond to a section called "Extras" in the changelog.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum SectionSource {
    CommitFooter(CommitFooter),
    CustomChangeType(CustomChangeType),
}

impl From<CommitFooter> for SectionSource {
    fn from(footer: CommitFooter) -> Self {
        Self::CommitFooter(footer)
    }
}

impl From<CustomChangeType> for SectionSource {
    fn from(change_type: CustomChangeType) -> Self {
        Self::CustomChangeType(change_type)
    }
}

impl Display for SectionSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommitFooter(footer) => footer.fmt(f),
            Self::CustomChangeType(change_type) => change_type.fmt(f),
        }
    }
}

/// A non-standard conventional commit type (e.g., the "doc" in "doc: some message")
/// or a non-standard changeset type ([`changesets::ChangeType::Custom`]).
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct CustomChangeType(String);

impl PartialEq<str> for CustomChangeType {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct CommitFooter(String);

impl Display for CommitFooter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for CommitFooter {
    fn from(token: &str) -> Self {
        Self(token.into())
    }
}

impl From<FooterToken<'_>> for CommitFooter {
    fn from(token: FooterToken) -> Self {
        Self(token.as_str().into())
    }
}

impl Display for CustomChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for CustomChangeType {
    fn from(token: String) -> Self {
        Self(token)
    }
}

impl From<&str> for CustomChangeType {
    fn from(token: &str) -> Self {
        Self(token.into())
    }
}

impl From<CustomChangeType> for String {
    fn from(custom: CustomChangeType) -> Self {
        custom.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sections(pub Vec<(SectionName, Vec<ChangeType>)>);

impl Sections {
    #[must_use]
    pub fn defaults() -> Vec<(SectionName, ChangeType)> {
        vec![
            (SectionName::from("Breaking Changes"), ChangeType::Breaking),
            (SectionName::from("Features"), ChangeType::Feature),
            (SectionName::from("Fixes"), ChangeType::Fix),
            ("Notes".into(), CommitFooter::from("Changelog-Note").into()),
        ]
    }

    pub fn iter(&self) -> impl Iterator<Item = &(SectionName, Vec<ChangeType>)> {
        self.0.iter()
    }

    #[allow(dead_code)]
    pub(crate) fn is_default(&self) -> bool {
        let defaults = Self::defaults();
        self.0.iter().enumerate().all(|(index, (name, sources))| {
            if sources.len() != 1 {
                return false;
            }
            sources.first().is_some_and(|source| {
                defaults
                    .get(index)
                    .is_some_and(|(default_name, default_source)| {
                        name == default_name && source == default_source
                    })
            })
        })
    }

    pub(crate) fn contains_footer(&self, footer: &git_conventional::Footer) -> bool {
        self.0.iter().any(|(_, sources)| {
            sources.iter().any(|source| match source {
                ChangeType::Custom(SectionSource::CommitFooter(footer_token)) => {
                    footer_token.0.eq_ignore_ascii_case(footer.token().as_str())
                }
                _ => false,
            })
        })
    }
}

impl Default for Sections {
    fn default() -> Self {
        Self(
            Self::defaults()
                .into_iter()
                .map(|(name, source)| (name, vec![source]))
                .collect(),
        )
    }
}

impl IntoIterator for Sections {
    type Item = (SectionName, Vec<ChangeType>);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct SectionName(String);

impl Display for SectionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for SectionName {
    fn from(token: &str) -> Self {
        Self(token.into())
    }
}

impl AsRef<str> for SectionName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
