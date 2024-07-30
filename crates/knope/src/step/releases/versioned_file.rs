use std::{fmt::Display, path::Path};

use knope_versioning::Version;

/// A version and where it came from.
#[derive(Clone)]
pub(crate) struct VersionFromSource {
    pub(crate) version: Version,
    pub(crate) source: VersionSource,
}

impl Display for VersionFromSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} from {}", self.version, self.source)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum VersionSource {
    OverrideVersion,
    File(String),
    Calculated,
}

impl Display for VersionSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionSource::OverrideVersion => write!(f, "--override-version option"),
            VersionSource::File(file) => write!(f, "file {file}"),
            VersionSource::Calculated => write!(f, "calculated by Knope"),
        }
    }
}

impl From<&Path> for VersionSource {
    fn from(path: &Path) -> Self {
        Self::File(path.display().to_string())
    }
}
