use relative_path::RelativePathBuf;
use toml_edit::{value, DocumentMut, TomlError};
use tracing::warn;

use crate::{semver::Version, Action};

/// Represents a Cargo.lock file.
#[derive(Clone, Debug)]
pub struct CargoLock {
    pub(super) path: RelativePathBuf,
    document: DocumentMut,
    diff: Vec<String>,
}

impl CargoLock {
    /// Parses the raw TOML without checking the format, yet.
    pub fn new(path: RelativePathBuf, toml: &str) -> Result<Self, Error> {
        let document: DocumentMut = toml.parse().map_err(|source| Error::Toml {
            source,
            path: path.clone(),
        })?;
        Ok(Self {
            path,
            document,
            diff: Vec::new(),
        })
    }

    pub fn set_version(
        mut self,
        new_version: &Version,
        dependency: Option<&str>,
    ) -> Result<Self, SetError> {
        let dependency = dependency.ok_or(SetError::MissingDependency)?;

        match self.document.get("version") {
            None => warn!("Unknown version of Cargo.lock, outcome may be unexpected"),
            Some(version)
                if version
                    .as_integer()
                    .is_some_and(|version| (3..=4).contains(&version)) => {}
            Some(version) => {
                warn!("Unsupported version of Cargo.lock: {version}. Outcome may be unexpected");
            }
        }

        let packages = self
            .document
            .get_mut("package")
            .and_then(|package| package.as_array_of_tables_mut())
            .ok_or_else(|| SetError::MissingPackageArray(self.path.clone()))?;
        for package in packages.iter_mut() {
            let name = package
                .get("name")
                .and_then(|name| name.as_str())
                .ok_or_else(|| SetError::MalformedPackage(self.path.clone()))?;
            if name != dependency {
                continue;
            }
            self.diff.push(format!("{name} = {new_version}"));
            match package.get_mut("version") {
                Some(version) => {
                    *version = value(new_version.to_string());
                }
                None => {
                    package.insert("version", value(new_version.to_string()));
                }
            }
        }
        Ok(self)
    }

    pub(super) fn write(self) -> Option<Action> {
        if self.diff.is_empty() {
            return None;
        }
        Some(Action::WriteToFile {
            content: self.document.to_string(),
            path: self.path,
            diff: self.diff.join(", "),
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum Error {
    #[error("Invalid TOML in {path}: {source}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(code(knope_versioning::cargo_lock::toml))
    )]
    Toml {
        path: RelativePathBuf,
        #[source]
        source: TomlError,
    },
}

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum SetError {
    #[error("Dependency was not specified when setting the version")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::cargo_lock::missing_dependency),
            help("This is likely a bug, please report it."),
            url("https://github.com/knope-dev/knope/issues"),
        )
    )]
    MissingDependency,
    #[error("{0} is expected to have an array of packages, but it does not")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(code(knope_versioning::cargo_lock::missing_package_array)),
        help("The Cargo.lock may be malformed, or Knope may not yet support a newer format.")
    )]
    MissingPackageArray(RelativePathBuf),
    #[error("Every package in {0} is expected to have a 'name' field containing a string, but one does not")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(code(knope_versioning::cargo_lock::missing_package_name)),
        help("The Cargo.lock may be malformed, or Knope may not yet support a newer format.")
    )]
    MalformedPackage(RelativePathBuf),
}
