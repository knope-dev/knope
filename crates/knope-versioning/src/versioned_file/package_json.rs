#[cfg(feature = "miette")]
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use serde::Deserialize;
use serde_json::{Map, Value};
use thiserror::Error;

use crate::{action::Action, semver::Version};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageJson {
    path: RelativePathBuf,
    raw: String,
    parsed: Json,
}

impl PackageJson {
    pub(crate) fn new(path: RelativePathBuf, content: String) -> Result<Self, Error> {
        match serde_json::from_str(&content) {
            Ok(parsed) => Ok(PackageJson {
                path,
                raw: content,
                parsed,
            }),
            Err(err) => Err(Error::Deserialize { path, source: err }),
        }
    }

    pub(crate) fn get_version(&self) -> &Version {
        &self.parsed.version
    }

    pub(crate) fn get_path(&self) -> &RelativePathBuf {
        &self.path
    }

    pub(crate) fn set_version(self, new_version: &Version) -> serde_json::Result<Action> {
        let mut json = serde_json::from_str::<Map<String, Value>>(&self.raw)?;
        json.insert(
            "version".to_string(),
            Value::String(new_version.to_string()),
        );
        let new_content = serde_json::to_string_pretty(&json)?;
        Ok(Action::WriteToFile {
            path: self.path,
            content: new_content,
            diff: new_version.to_string(),
        })
    }
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum Error {
    #[error("Error deserializing {path}: {source}")]
    #[cfg_attr(feature = "miette", diagnostic(
        code(package_json::deserialize),
        help("knope expects the package.json file to be an object with a top level `version` property"),
        url("https://knope.tech/reference/config-file/packages/#packagejson")
    ))]
    Deserialize {
        path: RelativePathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct Json {
    version: Version,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_get_version() {
        let content = r#"{
        "name": "tester",
        "version": "0.1.0-rc.0"
        }"#;

        assert_eq!(
            PackageJson::new(RelativePathBuf::new(), content.to_string())
                .unwrap()
                .get_version(),
            &Version::from_str("0.1.0-rc.0").unwrap()
        );
    }

    #[test]
    fn test_set_version() {
        let content = r#"{
        "name": "tester",
        "version": "0.1.0-rc.0"
        }"#;

        let new = PackageJson::new(RelativePathBuf::new(), content.to_string())
            .unwrap()
            .set_version(&Version::from_str("1.2.3-rc.4").unwrap())
            .unwrap();

        let expected = r#"{
  "name": "tester",
  "version": "1.2.3-rc.4"
}"#
        .to_string();
        let expected = Action::WriteToFile {
            path: RelativePathBuf::new(),
            content: expected,
            diff: "1.2.3-rc.4".to_string(),
        };
        assert_eq!(new, expected);
    }

    #[test]
    fn retain_property_order() {
        let content = r#"{
        "name": "tester",
        "version": "0.1.0-rc.0",
        "dependencies": {}
        }"#;

        let new = PackageJson::new(RelativePathBuf::new(), content.to_string())
            .unwrap()
            .set_version(&Version::from_str("1.2.3-rc.4").unwrap())
            .unwrap();

        let expected = r#"{
  "name": "tester",
  "version": "1.2.3-rc.4",
  "dependencies": {}
}"#
        .to_string();
        let expected = Action::WriteToFile {
            path: RelativePathBuf::new(),
            content: expected,
            diff: "1.2.3-rc.4".to_string(),
        };
        assert_eq!(new, expected);
    }
}
