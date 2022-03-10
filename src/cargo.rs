use std::path::Path;

use color_eyre::Result;
use serde::Deserialize;
use toml::Spanned;

pub(crate) fn get_version<P: AsRef<Path>>(path: P) -> Option<String> {
    Some(
        toml::from_str::<Cargo>(&std::fs::read_to_string(path).ok()?)
            .ok()?
            .package
            .version
            .into_inner(),
    )
}

pub(crate) fn set_version<P: AsRef<Path>>(path: P, new_version: &str) -> Result<()> {
    let mut toml = std::fs::read_to_string(&path)?;
    let doc: Cargo = toml::from_str(&toml)?;

    // Account for quotes with +- 1
    let start = doc.package.version.start() + 1;
    let end = doc.package.version.end() - 1;

    toml.replace_range(start..end, new_version);

    std::fs::write(path, toml)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct Cargo {
    package: Package,
}

#[derive(Debug, Deserialize)]
struct Package {
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
        [package]
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
        [package]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;
        std::fs::write(&file, content).unwrap();

        set_version(&file, "1.2.3-rc.4").unwrap();

        let expected = content.replace("0.1.0-rc.0", "1.2.3-rc.4");
        assert_eq!(std::fs::read_to_string(file).unwrap(), expected);
    }
}
