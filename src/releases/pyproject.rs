use serde::Deserialize;
use toml::Spanned;

use crate::step::StepError;

pub(crate) fn get_version(content: &str) -> Result<String, StepError> {
    toml::from_str::<PyProject>(content)
        .map_err(|_| StepError::InvalidPyProject)
        .map(|pyproject| pyproject.tool.poetry.version.into_inner())
}

pub(crate) fn set_version(
    mut pyproject_toml: String,
    new_version: &str,
) -> Result<String, StepError> {
    let doc: PyProject =
        toml::from_str(&pyproject_toml).map_err(|_| StepError::InvalidPyProject)?;

    // Account for quotes around value with +- 1
    let start = doc.tool.poetry.version.start() + 1;
    let end = doc.tool.poetry.version.end() - 1;

    pyproject_toml.replace_range(start..end, new_version);
    Ok(pyproject_toml)
}

#[derive(Debug, Deserialize)]
struct PyProject {
    tool: Tool,
}

#[derive(Debug, Deserialize)]
struct Tool {
    poetry: Poetry,
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
        [tool.poetry]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;

        assert_eq!(get_version(content).unwrap(), "0.1.0-rc.0".to_string());
    }

    #[test]
    fn test_set_version() {
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
