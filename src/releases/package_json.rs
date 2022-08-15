use serde::Deserialize;
use serde_json::{Map, Value};

pub(crate) fn get_version(content: &str) -> Result<String, serde_json::Error> {
    serde_json::from_str::<Package>(content).map(|package| package.version)
}

pub(crate) fn set_version(
    package_json: &str,
    new_version: &str,
) -> Result<String, serde_json::Error> {
    let mut json = serde_json::from_str::<Map<String, Value>>(package_json)?;
    json.insert(
        "version".to_string(),
        Value::String(new_version.to_string()),
    );
    serde_json::to_string_pretty(&json)
}

#[derive(Debug, Deserialize)]
struct Package {
    version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_version() {
        let content = r###"{
        "name": "tester",
        "version": "0.1.0-rc.0"
        }"###;

        assert_eq!(get_version(content).unwrap(), "0.1.0-rc.0".to_string())
    }

    #[test]
    fn test_set_version() {
        let content = r###"{
        "name": "tester",
        "version": "0.1.0-rc.0"
        }"###;

        let new = set_version(content, "1.2.3-rc.4").unwrap();

        let expected = r###"{
  "name": "tester",
  "version": "1.2.3-rc.4"
}"###
            .to_string();
        assert_eq!(new, expected);
    }

    #[test]
    fn retain_property_order() {
        let content = r###"{
        "name": "tester",
        "version": "0.1.0-rc.0",
        "dependencies": {}
        }"###;

        let new = set_version(content, "1.2.3-rc.4").unwrap();

        let expected = r###"{
  "name": "tester",
  "version": "1.2.3-rc.4",
  "dependencies": {}
}"###
            .to_string();
        assert_eq!(new, expected);
    }
}
