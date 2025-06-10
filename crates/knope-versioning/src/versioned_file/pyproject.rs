use std::str::FromStr;

#[cfg(feature = "miette")]
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use serde::Deserialize;
use thiserror::Error;
use toml::Spanned;

use crate::{action::Action, semver::Version};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PyProject {
    pub(super) path: RelativePathBuf,
    raw_toml: String,
    parsed: Toml,
    pub(super) version: Version,
    diff: Option<String>,
}

impl PyProject {
    pub(crate) fn new(path: RelativePathBuf, raw_toml: String) -> Result<Self, Error> {
        match toml::from_str::<Toml>(&raw_toml) {
            Ok(parsed) => parsed
                .version(&path)
                .and_then(|version| Version::from_str(version).map_err(Error::from))
                .map(|version| PyProject {
                    path,
                    raw_toml,
                    parsed,
                    version,
                    diff: None,
                }),
            Err(err) => Err(Error::Deserialization(path, Box::new(err))),
        }
    }

    pub(crate) fn set_version(mut self, new_version: &Version) -> Self {
        let version_str = new_version.to_string();
        let (poetry_version, project_version) = self.parsed.versions();

        for version in [poetry_version, project_version].into_iter().flatten() {
            // Account for quotes around value with +- 1
            let start = version.span().start + 1;
            let end = version.span().end - 1;
            self.raw_toml.replace_range(start..end, &version_str);
        }
        self.diff = Some(version_str);
        self
    }

    pub(crate) fn write(self) -> Option<Action> {
        self.diff.map(|diff| Action::WriteToFile {
            content: self.raw_toml,
            path: self.path,
            diff,
        })
    }
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum Error {
    #[error("Could not deserialize {0} as a pyproject.toml: {1}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(pyproject::invalid),
            help(
                "knope expects the pyproject.toml file to have a `tool.poetry.version` or \
                        `project.version` property. If you use a different location for your version, please \
                        open an issue to add support."
            ),
            url("https://knope.tech/reference/config-file/packages/#pyprojecttoml")
        )
    )]
    Deserialization(RelativePathBuf, #[source] Box<toml::de::Error>),
    #[error("Found conflicting versions {project} and {poetry} in {path}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(pyproject::inconsistent),
            help("Make sure [project.version] and [tool.poetry.version] are the same."),
            url("https://knope.tech/reference/config-file/packages/#pyprojecttoml")
        )
    )]
    InconsistentVersions {
        project: String,
        poetry: String,
        path: RelativePathBuf,
    },
    #[error("No versions were found in {0}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(pyproject::no_versions),
            help("Make sure [project.version] or [tool.poetry.version] is set."),
            url("https://knope.tech/reference/config-file/packages/#pyprojecttoml")
        )
    )]
    NoVersions(RelativePathBuf),
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    Semver(#[from] crate::semver::Error),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct Toml {
    tool: Option<Tool>,
    project: Option<Metadata>,
}

impl Toml {
    /// Get the consistent version from `pyproject.toml` or error.
    /// `path` is used for better error messages.
    fn version(&self, path: &RelativePathBuf) -> Result<&str, Error> {
        let (poetry_version, project_version) = self.versions();

        match (poetry_version, project_version) {
            (Some(poetry_version), Some(project_version)) => {
                if poetry_version == project_version {
                    Ok(poetry_version.as_ref())
                } else {
                    Err(Error::InconsistentVersions {
                        poetry: poetry_version.as_ref().to_string(),
                        project: project_version.as_ref().to_string(),
                        path: path.into(),
                    })
                }
            }
            (Some(poetry_version), None) => Ok(poetry_version.as_ref()),
            (None, Some(project_version)) => Ok(project_version.as_ref()),
            (None, None) => Err(Error::NoVersions(path.clone())),
        }
    }

    fn versions(&self) -> (Option<&Spanned<String>>, Option<&Spanned<String>>) {
        let poetry_version = self
            .tool
            .as_ref()
            .and_then(|tool| tool.poetry.as_ref()?.version.as_ref());
        let project_version = self
            .project
            .as_ref()
            .and_then(|project| project.version.as_ref());
        (poetry_version, project_version)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct Tool {
    poetry: Option<Metadata>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct Metadata {
    version: Option<Spanned<String>>,
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_get_version_poetry() {
        let content = r#"
        [tool.poetry]
        name = "tester"
        version = "0.1.0-rc.0"
        "#;

        assert_eq!(
            PyProject::new(RelativePathBuf::new(), content.to_string())
                .unwrap()
                .version,
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
            PyProject::new(RelativePathBuf::new(), content.to_string())
                .unwrap()
                .version,
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
            PyProject::new(RelativePathBuf::new(), content.to_string())
                .unwrap()
                .version,
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

        match PyProject::new(RelativePathBuf::new(), content.to_string()) {
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

        let pyproject =
            PyProject::new(RelativePathBuf::from("beep/boop"), String::from(content)).unwrap();
        let action = pyproject
            .set_version(&Version::from_str("1.2.3-rc.4").unwrap())
            .write()
            .expect("Diff to write");

        let expected = Action::WriteToFile {
            content: r#"
        [tool.poetry]
        name = "tester"
        version = "1.2.3-rc.4"
        
        [project]
        name = "tester"
        version = "1.2.3-rc.4"
        "#
            .to_string(),
            path: RelativePathBuf::from("beep/boop"),
            diff: "1.2.3-rc.4".to_string(),
        };
        assert_eq!(action, expected);
    }
}
