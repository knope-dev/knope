use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Represents a Package in the `[[packages]]` section of `knope.toml`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Package {
    /// The files which define the current version of the package.
    pub(crate) versioned_files: Vec<PathBuf>,
    /// The path to the `CHANGELOG.md` file (if any) to be updated when running [`crate::Step::PrepareRelease`].
    pub(crate) changelog: Option<String>,
}
