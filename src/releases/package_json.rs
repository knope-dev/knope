use serde::Deserialize;
use serde_json::Value;

use crate::step::StepError;

pub(crate) fn get_version(content: &str) -> Result<String, StepError> {
    serde_json::from_str::<Package>(content)
        .map_err(|_| StepError::InvalidPackageJson)
        .map(|package| package.version)
}

pub(crate) fn set_version(package_json: &str, new_version: &str) -> Result<String, StepError> {
    let json = match serde_json::from_str::<Value>(package_json)
        .map_err(|_| StepError::InvalidPackageJson)?
    {
        Value::Object(mut data) => {
            data.insert(
                "version".to_string(),
                Value::String(new_version.to_string()),
            );
            Some(Value::Object(data))
        }
        _ => None,
    }
    .ok_or(StepError::InvalidPackageJson)?;
    serde_json::to_string_pretty(&json).map_err(|e| StepError::Bug(Box::new(e)))
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
