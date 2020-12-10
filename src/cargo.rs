use std::path::Path;

use color_eyre::Result;
use serde::Deserialize;
use toml_edit::{value, Document};

pub(crate) fn get_version<P: AsRef<Path>>(path: P) -> Option<String> {
    Some(
        toml::from_str::<Cargo>(&std::fs::read_to_string(path).ok()?)
            .ok()?
            .package
            .version,
    )
}

pub(crate) fn set_version<P: AsRef<Path>>(path: P, new_version: &str) -> Result<()> {
    let toml = std::fs::read_to_string(&path)?;
    let mut doc = toml.parse::<Document>()?;
    doc["package"]["version"] = value(new_version);
    std::fs::write(path, doc.to_string())?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct Cargo {
    package: Package,
}

#[derive(Debug, Deserialize)]
struct Package {
    version: String,
}

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn test_get_version() {
        let file = NamedTempFile::new().unwrap();
        let content = r###"
        [package]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;
        std::fs::write(&file, content).unwrap();

        assert_eq!(get_version(file), Some("0.1.0-rc.0".to_string()))
    }

    #[test]
    fn test_set_version() {
        let file = NamedTempFile::new().unwrap();
        let content = r###"
        [package]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;
        std::fs::write(&file, content).unwrap();

        set_version(&file, "1.2.3-rc.4").unwrap();

        let expected = r###"
        [package]
        name = "tester"
        version = "1.2.3-rc.4"
        "###
        .to_string();
        assert_eq!(std::fs::read_to_string(file).unwrap(), expected);
    }
}
