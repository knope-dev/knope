use std::path::Path;

use serde::Deserialize;
use toml::Spanned;

use crate::step::StepError;

/// Extrat the consistent version from a `pyproject.toml` file's content or return an error.
///
/// `path` is used for error reporting.
pub(crate) fn get_version(content: &str, path: &Path) -> Result<String, StepError> {
    toml::from_str::<PyProject>(content)
        .map_err(|_| StepError::InvalidPyProject(path.into()))
        .and_then(|pyproject| pyproject.version(path))
}

/// Replace the version(s) in a `pyproject.toml` file's content with `new_version` or return an error.
///
/// `path` is used for error reporting.
pub(crate) fn set_version(
    pyproject_toml: String,
    new_version: &str,
    path: &Path,
) -> Result<String, StepError> {
    toml::from_str(&pyproject_toml)
        .map_err(|_| StepError::InvalidPyProject(path.into()))
        .map(|pyproject: PyProject| pyproject.set_version(pyproject_toml, new_version))
}

#[derive(Debug, Deserialize)]
struct PyProject {
    tool: Option<Tool>,
    project: Option<Metadata>,
}

impl PyProject {
    /// Get the consistent version from `pyproject.toml` or error.
    /// `path` is used for better error messages.
    fn version(self, path: &Path) -> Result<String, StepError> {
        let (poetry_version, project_version) = self.into_versions();

        match (poetry_version, project_version) {
            (Some(poetry_version), Some(project_version)) => {
                if poetry_version == project_version {
                    Ok(poetry_version.into_inner())
                } else {
                    Err(StepError::InconsistentVersions(
                        poetry_version.into_inner(),
                        project_version.into_inner(),
                    ))
                }
            }
            (Some(poetry_version), None) => Ok(poetry_version.into_inner()),
            (None, Some(project_version)) => Ok(project_version.into_inner()),
            (None, None) => Err(StepError::InvalidPyProject(path.into())),
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
            let start = version.start() + 1;
            let end = version.end() - 1;
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

    #[test]
    fn test_get_version_poetry() {
        let content = r###"
        [tool.poetry]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;

        assert_eq!(
            get_version(content, PathBuf::new().as_path()).unwrap(),
            "0.1.0-rc.0".to_string()
        );
    }

    #[test]
    fn test_get_version_pep621() {
        let content = r###"
        [project]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;

        assert_eq!(
            get_version(content, PathBuf::new().as_path()).unwrap(),
            "0.1.0-rc.0".to_string()
        );
    }

    #[test]
    fn test_get_version_mixed() {
        let content = r###"
        [tool.poetry]
        name = "tester"
        version = "0.1.0-rc.0"
        
        [project]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;

        assert_eq!(
            get_version(content, PathBuf::new().as_path()).unwrap(),
            "0.1.0-rc.0".to_string()
        );
    }

    #[test]
    fn test_get_version_mismatch() {
        let content = r###"
        [tool.poetry]
        name = "tester"
        version = "1.0.0"
        
        [project]
        name = "tester"
        version = "2.3.4"
        "###;

        match get_version(content, PathBuf::new().as_path()) {
            Err(StepError::InconsistentVersions(first, second)) => {
                assert_eq!(first, "1.0.0".to_string());
                assert_eq!(second, "2.3.4".to_string());
            }
            other => panic!("Unexpected result {other:?}"),
        }
    }

    #[test]
    fn test_set_version() {
        let content = r###"
        [tool.poetry]
        name = "tester"
        version = "0.1.0-rc.0"
        
        [project]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;

        let new = set_version(String::from(content), "1.2.3-rc.4", &PathBuf::new()).unwrap();

        let expected = r###"
        [tool.poetry]
        name = "tester"
        version = "1.2.3-rc.4"
        
        [project]
        name = "tester"
        version = "1.2.3-rc.4"
        "###
        .to_string();
        assert_eq!(new, expected);
    }
}
