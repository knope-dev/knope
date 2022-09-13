use serde::Deserialize;
use thiserror::Error;
use toml::Spanned;

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("No package version given in [project] or [poetry.tools] table")]
    MissingVersion,

    #[error("Could not deserialize pyproject.toml: {0}")]
    Deserialize(#[from] toml::de::Error),
}

pub(crate) fn get_version(content: &str) -> Result<String, Error> {
    let pyproject = toml::from_str::<PyProject>(content)?;
    pyproject
        .project
        .map(|project| project.version.into_inner())
        .or_else(|| pyproject.tool.map(|tool| tool.poetry.version.into_inner()))
        .ok_or(Error::MissingVersion)
}

pub(crate) fn set_version(
    mut pyproject_toml: String,
    new_version: &str,
) -> Result<String, toml::de::Error> {
    let doc: PyProject = toml::from_str(&pyproject_toml)?;

    if let Some(project) = &doc.project {
        // Account for quotes around value with +- 1
        let start = project.version.start() + 1;
        let end = project.version.end() - 1;
        pyproject_toml.replace_range(start..end, new_version);
    }

    if let Some(tool) = &doc.tool {
        let start = tool.poetry.version.start() + 1;
        let end = tool.poetry.version.end() - 1;
        pyproject_toml.replace_range(start..end, new_version);
    }

    Ok(pyproject_toml)
}

#[derive(Debug, Deserialize)]
struct PyProject {
    tool: Option<Tool>,
    project: Option<Project>,
}

#[derive(Debug, Deserialize)]
struct Tool {
    poetry: Poetry,
}

#[derive(Debug, Deserialize)]
struct Project {
    version: Spanned<String>,
}

#[derive(Debug, Deserialize)]
struct Poetry {
    version: Spanned<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_version() {
        let content = r###"
        [project]
        name = "tester"
        version = "0.1.0-rc.0"

        [tool.poetry]
        name = "tester"
        version = "1.2.3-rc.0"
        "###;

        assert_eq!(get_version(content).unwrap(), "0.1.0-rc.0".to_string());
    }

    #[test]
    fn test_get_version_project_only() {
        let content = r###"
        [project]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;

        assert_eq!(get_version(content).unwrap(), "0.1.0-rc.0".to_string());
    }

    #[test]
    fn test_get_version_poetry_only() {
        let content = r###"
        [tool.poetry]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;

        assert_eq!(get_version(content).unwrap(), "0.1.0-rc.0".to_string());
    }

    #[test]
    fn test_set_version() {
        let content = r###"
        [project]
        name = "tester"
        version = "0.1.0-rc.0"

        [tool.poetry]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;

        let new = set_version(String::from(content), "1.2.3-rc.4").unwrap();

        let expected = r###"
        [project]
        name = "tester"
        version = "1.2.3-rc.4"

        [tool.poetry]
        name = "tester"
        version = "1.2.3-rc.4"
        "###
        .to_string();
        assert_eq!(new, expected);
    }

    #[test]
    fn test_set_version_project_only() {
        let content = r###"
        [project]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;

        let new = set_version(String::from(content), "1.2.3-rc.4").unwrap();

        let expected = r###"
        [project]
        name = "tester"
        version = "1.2.3-rc.4"
        "###
        .to_string();
        assert_eq!(new, expected);
    }

    #[test]
    fn test_set_version_poetry_only() {
        let content = r###"
        [tool.poetry]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;

        let new = set_version(String::from(content), "1.2.3-rc.4").unwrap();

        let expected = r###"
        [tool.poetry]
        name = "tester"
        version = "1.2.3-rc.4"
        "###
        .to_string();
        assert_eq!(new, expected);
    }
}
