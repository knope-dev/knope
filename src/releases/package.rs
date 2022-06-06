use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Represents a Package in the `[[packages]]` section of `knope.toml`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Package {
    /// The files which define the current version of the package.
    pub(crate) versioned_files: Vec<PathBuf>,
    /// The path to the `CHANGELOG.md` file (if any) to be updated when running [`crate::Step::PrepareRelease`].
    pub(crate) changelog: Option<String>,
}

const SUPPORTED_FORMATS: [&str; 3] = ["Cargo.toml", "pyproject.toml", "package.json"];

/// Find the first supported package manager in the current directory that can be added to generated config.
pub(crate) fn find_packages() -> Vec<Package> {
    let changelog = if Path::exists(&PathBuf::from("CHANGELOG.md")) {
        Some(String::from("CHANGELOG.md"))
    } else {
        None
    };

    for supported in SUPPORTED_FORMATS.map(PathBuf::from) {
        if Path::exists(&supported) {
            return vec![Package {
                versioned_files: vec![supported],
                changelog,
            }];
        }
    }
    return vec![];
}

/// Includes some helper text for the user to understand how to use the config to define packages.
pub(crate) fn suggested_package_toml() -> String {
    let packages = find_packages();
    if packages.is_empty() {
        return format!(
                "No supported package managers found in current directory. \
                The supported formats are {formats}. Here's how you might define a package for `Cargo.toml`:\
                \n\n```\n[[packages]]\nversioned_files = [\"Cargo.toml\"]\nchangelog = \"CHANGELOG.md\"\n```",
                formats = SUPPORTED_FORMATS.join(", ")
            );
    }
    return format!(
        "Found the package metadata file {file} in the current directory. You may need to add this \
        to your knope.toml:\n\n```\n[[packages]]\n{toml}```",
        file = packages[0].versioned_files[0].to_str().unwrap(),
        toml = toml::to_string(&packages[0]).unwrap()
    );
}
