use std::path::Path;

use color_eyre::eyre::eyre;
use color_eyre::Result;
use serde::Deserialize;

pub(crate) fn get_version<P: AsRef<Path>>(path: P) -> Option<String> {
    Some(
        toml::from_str::<Cargo>(&std::fs::read_to_string(path).ok()?)
            .ok()?
            .package
            .version,
    )
}

pub(crate) fn set_version<P: AsRef<Path>>(path: P, new_version: String) -> Result<()> {
    let mut toml: toml::Value = toml::from_str(&std::fs::read_to_string(&path)?)?;
    toml.get_mut("package")
        .ok_or_else(|| eyre!("TOML missing package key"))?
        .as_table_mut()
        .ok_or_else(|| eyre!("TOML package key was not a table"))?
        .insert("version".to_string(), toml::Value::String(new_version));
    std::fs::write(path, toml::to_string_pretty(&toml)?)?;
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

        set_version(&file, "1.2.3-rc.4".to_string()).unwrap();

        let expected = r###"[package]
name = 'tester'
version = '1.2.3-rc.4'
"###
        .to_string();
        assert_eq!(std::fs::read_to_string(file).unwrap(), expected);
    }
}
