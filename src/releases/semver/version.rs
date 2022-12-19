use std::fmt::Display;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::step::StepError;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Version {
    pub(crate) major: u64,
    pub(crate) minor: u64,
    pub(crate) patch: u64,
    pub(crate) pre: Option<Prerelease>,
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
            .then({
                match (&self.pre, &other.pre) {
                    (Some(pre), Some(other_pre)) => pre.cmp(other_pre),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl FromStr for Version {
    type Err = StepError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (version, pre) = s
            .split_once('-')
            .map_or((s, None), |(version, pre)| (version, Some(pre)));
        let version_parts = version
            .split('.')
            .map(|part| {
                part.parse::<u64>()
                    .map_err(|err| StepError::InvalidSemanticVersion(err.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if version_parts.len() != 3 {
            return Err(StepError::InvalidSemanticVersion(
                "Version must have 3 parts".to_string(),
            ));
        }
        Ok(Self {
            major: version_parts[0],
            minor: version_parts[1],
            patch: version_parts[2],
            pre: pre.map(Prerelease::from_str).transpose()?,
        })
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre {
            write!(f, "-{pre}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Prerelease {
    pub(crate) label: Label,
    pub(crate) version: u64,
}

impl Display for Prerelease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.label, self.version)
    }
}

impl FromStr for Prerelease {
    type Err = StepError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (label, version) = s
            .split_once('.')
            .ok_or_else(|| StepError::InvalidSemanticVersion("Invalid prerelease".to_string()))?;
        Ok(Self {
            label: Label(String::from(label)),
            version: version
                .parse::<u64>()
                .map_err(|err| StepError::InvalidSemanticVersion(err.to_string()))?,
        })
    }
}

impl Ord for Prerelease {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.label
            .cmp(&other.label)
            .then(self.version.cmp(&other.version))
    }
}

impl PartialOrd for Prerelease {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Prerelease {
    pub(crate) fn new(label: Label, version: u64) -> Self {
        Self { label, version }
    }
}

/// The label component of a Prerelease (e.g., "alpha" in "1.0.0-alpha.1").
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[repr(transparent)]
pub(crate) struct Label(pub(crate) String);

impl Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Label {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}
