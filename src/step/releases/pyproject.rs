use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use miette::Diagnostic;
use serde::Deserialize;
use thiserror::Error;
use toml::Spanned;

use super::{semver, semver::Version};
use crate::{dry_run::DryRun, fs};

/// Extract the consistent version from a `pyproject.toml` file's content or return an error.
///
/// `path` is used for error reporting.
pub(crate) fn get_version(content: &str, path: &Path) -> Result<Version, Error> {
    toml::from_str::<PyProject>(content)
        .map_err(|source| Error::Deserialization(path.into(), source))
        .and_then(|pyproject| pyproject.version(path))
        .and_then(|version| Version::from_str(&version).map_err(Error::from))
}

/// Replace the version(s) in a `pyproject.toml` file's content with `new_version` or return an error.
///
/// `path` is used for error reporting.
pub(crate) fn set_version(
    dry_run: DryRun,
    pyproject_toml: String,
    new_version: &Version,
    path: &Path,
) -> Result<String, Error> {
    let version_str = new_version.to_string();
    let contents = toml::from_str(&pyproject_toml)
        .map_err(|source| Error::Deserialization(path.into(), source))
        .map(|pyproject: PyProject| pyproject.set_version(pyproject_toml, &version_str))?;
    fs::write(dry_run, &version_str, path, &contents)?;
    Ok(contents)
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
    #[error("Could not deserialize {0} as a pyproject.toml: {1}")]
    #[diagnostic(
        code(pyproject::invalid),
        help(
        "knope expects the pyproject.toml file to have a `tool.poetry.version` or \
                    `project.version` property. If you use a different location for your version, please \
                    open an issue to add support."
        ),
        url("https://knope-dev.github.io/knope/config/packages.html#supported-formats-for-versioning")
    )]
    Deserialization(PathBuf, #[source] toml::de::Error),
    #[error("Found conflicting versions {project} and {poetry} in {path}")]
    #[diagnostic(
        code(pyproject::inconsistent),
        help("Make sure [project.version] and [tool.poetry.version] are the same."),
        url("https://knope-dev.github.io/knope/config/packages.html#supported-formats-for-versioning")
    )]
    InconsistentVersions {
        project: String,
        poetry: String,
        path: PathBuf,
    },
    #[error("No versions were found in {0}")]
    #[diagnostic(
        code(pyproject::no_versions),
        help("Make sure [project.version] or [tool.poetry.version] is set."),
        url("https://knope-dev.github.io/knope/config/packages.html#supported-formats-for-versioning")
    )]
    NoVersions(PathBuf),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Semver(#[from] semver::version::Error),
}

#[derive(Debug, Deserialize)]
struct PyProject {
    tool: Option<Tool>,
    project: Option<Metadata>,
}

impl PyProject {
    /// Get the consistent version from `pyproject.toml` or error.
    /// `path` is used for better error messages.
    fn version(self, path: &Path) -> Result<String, Error> {
        let (poetry_version, project_version) = self.into_versions();

        match (poetry_version, project_version) {
            (Some(poetry_version), Some(project_version)) => {
                if poetry_version == project_version {
                    Ok(poetry_version.into_inner())
                } else {
                    Err(Error::InconsistentVersions {
                        poetry: poetry_version.into_inner(),
                        project: project_version.into_inner(),
                        path: path.into(),
                    })
                }
            }
            (Some(poetry_version), None) => Ok(poetry_version.into_inner()),
            (None, Some(project_version)) => Ok(project_version.into_inner()),
            (None, None) => Err(Error::NoVersions(path.into())),
        }
    }

    fn into_versions(self) -> (Option<Spanned<String>>, Option<Spanned<String>>) {
        let poetry_version = self
            .tool
            .and_then(|tool| tool.poetry)
            .and_then(|poetry| poetry.version);
        let project_version = self.project.and_then(|project| project.version);
        (poetry_version, project_version)
    }

    /// Replace the version(s) in the file's content with `new_version`.
    ///
    /// Uses the inner spans of the parsed TOML to determine where the replace contents.
    fn set_version(self, mut raw_contents: String, new_version: &str) -> String {
        let (poetry_version, project_version) = self.into_versions();

        for version in [poetry_version, project_version].into_iter().flatten() {
            // Account for quotes around value with +- 1
            let start = version.span().start + 1;
            let end = version.span().end - 1;
            raw_contents.replace_range(start..end, new_version);
        }
        raw_contents
    }
}

#[derive(Debug, Deserialize)]
struct Tool {
    poetry: Option<Metadata>,
}

#[derive(Debug, Deserialize)]
struct Metadata {
    version: Option<Spanned<String>>,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::dry_run::fake_dry_run;

    #[test]
    fn test_get_version_poetry() {
        let content = r#"
        [tool.poetry]
        name = "tester"
        version = "0.1.0-rc.0"
        "#;

        assert_eq!(
            get_version(content, PathBuf::new().as_path()).unwrap(),
            Version::from_str("0.1.0-rc.0").unwrap()
        );
    }

    #[test]
    fn test_get_version_pep621() {
        let content = r#"
        [project]
        name = "tester"
        version = "0.1.0-rc.0"
        "#;

        assert_eq!(
            get_version(content, PathBuf::new().as_path()).unwrap(),
            Version::from_str("0.1.0-rc.0").unwrap()
        );
    }

    #[test]
    fn test_get_version_mixed() {
        let content = r#"
        [tool.poetry]
        name = "tester"
        version = "0.1.0-rc.0"
        
        [project]
        name = "tester"
        version = "0.1.0-rc.0"
        "#;

        assert_eq!(
            get_version(content, PathBuf::new().as_path()).unwrap(),
            Version::from_str("0.1.0-rc.0").unwrap()
        );
    }

    #[test]
    fn test_get_version_mismatch() {
        let content = r#"
        [tool.poetry]
        name = "tester"
        version = "1.0.0"
        
        [project]
        name = "tester"
        version = "2.3.4"
        "#;

        match get_version(content, PathBuf::new().as_path()) {
            Err(Error::InconsistentVersions {
                poetry, project, ..
            }) => {
                assert_eq!(poetry, "1.0.0".to_string());
                assert_eq!(project, "2.3.4".to_string());
            }
            other => panic!("Unexpected result {other:?}"),
        }
    }

    #[test]
    fn test_set_version() {
        let content = r#"
        [tool.poetry]
        name = "tester"
        version = "0.1.0-rc.0"
        
        [project]
        name = "tester"
        version = "0.1.0-rc.0"
        "#;

        let new = set_version(
            &mut fake_dry_run(),
            String::from(content),
            &Version::from_str("1.2.3-rc.4").unwrap(),
            &PathBuf::new(),
        )
        .unwrap();

        let expected = r#"
        [tool.poetry]
        name = "tester"
        version = "1.2.3-rc.4"
        
        [project]
        name = "tester"
        version = "1.2.3-rc.4"
        "#
        .to_string();
        assert_eq!(new, expected);
    }
}
