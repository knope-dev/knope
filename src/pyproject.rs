use std::path::Path;

use serde::Deserialize;
use toml::Spanned;

use crate::step::StepError;

pub(crate) fn get_version<P: AsRef<Path>>(path: P) -> Option<String> {
    Some(
        toml::from_str::<PyProject>(&std::fs::read_to_string(path).ok()?)
            .ok()?
            .tool
            .poetry
            .version
            .into_inner(),
    )
}

pub(crate) fn set_version<P: AsRef<Path>>(path: P, new_version: &str) -> Result<(), StepError> {
    let mut toml = std::fs::read_to_string(&path)?;
    let doc: PyProject = toml::from_str(&toml).map_err(|_| StepError::InvalidPyProject)?;

    // Account for quotes around value with +- 1
    let start = doc.tool.poetry.version.start() + 1;
    let end = doc.tool.poetry.version.end() - 1;

    toml.replace_range(start..end, new_version);
    std::fs::write(path, toml)?;
    Ok(())
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
    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn test_get_version() {
        let file = NamedTempFile::new().unwrap();
        let content = r###"
        [tool.poetry]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;
        std::fs::write(&file, content).unwrap();

        assert_eq!(get_version(file), Some("0.1.0-rc.0".to_string()));
    }

    #[test]
    fn test_set_version() {
        let file = NamedTempFile::new().unwrap();
        let content = r###"
        [tool.poetry]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;
        std::fs::write(&file, content).unwrap();

        set_version(&file, "1.2.3-rc.4").unwrap();

        let expected = r###"
        [tool.poetry]
        name = "tester"
        version = "1.2.3-rc.4"
        "###
        .to_string();
        assert_eq!(std::fs::read_to_string(file).unwrap(), expected);
    }
}
