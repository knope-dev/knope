use std::path::Path;

use color_eyre::eyre::{eyre, Result};
use serde::Deserialize;
use serde_json::Value;

pub(crate) fn get_version<P: AsRef<Path>>(path: P) -> Option<String> {
    Some(
        serde_json::from_str::<Package>(&std::fs::read_to_string(path).ok()?)
            .ok()?
            .version,
    )
}

pub(crate) fn set_version<P: AsRef<Path>>(path: P, new_version: &str) -> Result<()> {
    let json = std::fs::read_to_string(&path)?;
    let json = match serde_json::from_str::<Value>(&json)? {
        Value::Object(mut data) => {
            data.insert(
                "version".to_string(),
                Value::String(new_version.to_string()),
            );
            Some(Value::Object(data))
        }
        _ => None,
    }
    .ok_or_else(|| eyre!("Invalid package.json contents"))?;
    std::fs::write(path, serde_json::to_string_pretty(&json)?)?;
    Ok(())
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
        let content = r###"{
        "name": "tester",
        "version": "0.1.0-rc.0"
        }"###;
        std::fs::write(&file, content).unwrap();

        assert_eq!(get_version(file), Some("0.1.0-rc.0".to_string()))
    }

    #[test]
    fn test_set_version() {
        let file = NamedTempFile::new().unwrap();
        let content = r###"{
        "name": "tester",
        "version": "0.1.0-rc.0"
        }"###;
        std::fs::write(&file, content).unwrap();

        set_version(&file, "1.2.3-rc.4").unwrap();

        let expected = r###"{
  "name": "tester",
  "version": "1.2.3-rc.4"
}"###
            .to_string();
        assert_eq!(std::fs::read_to_string(file).unwrap(), expected);
    }

    #[test]
    fn retain_property_order() {
        let file = NamedTempFile::new().unwrap();
        let content = r###"{
        "name": "tester",
        "version": "0.1.0-rc.0",
        "dependencies": {}
        }"###;
        std::fs::write(&file, content).unwrap();

        set_version(&file, "1.2.3-rc.4").unwrap();

        let expected = r###"{
  "name": "tester",
  "version": "1.2.3-rc.4",
  "dependencies": {}
}"###
            .to_string();
        assert_eq!(std::fs::read_to_string(file).unwrap(), expected);
    }
}
