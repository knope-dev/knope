#[cfg(feature = "miette")]
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use serde::Deserialize;
use serde_json::{Map, Value};
use thiserror::Error;

use crate::{action::Action, semver::Version};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TauriConfJson {
    path: RelativePathBuf,
    raw: String,
    parsed: Json,
    diff: Option<String>,
}

impl TauriConfJson {
    pub(crate) fn new(path: RelativePathBuf, content: String) -> Result<Self, Error> {
        match serde_json::from_str(&content) {
            Ok(parsed) => Ok(TauriConfJson {
                path,
                raw: content,
                parsed,
                diff: None,
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

    pub(crate) fn set_version(mut self, new_version: &Version) -> serde_json::Result<Self> {
        let mut json = serde_json::from_str::<Map<String, Value>>(&self.raw)?;
        json.insert(
            "version".to_string(),
            Value::String(new_version.to_string()),
        );
        self.raw = serde_json::to_string_pretty(&json)?;
        self.diff = Some(new_version.to_string());
        Ok(self)
    }

    pub(crate) fn write(self) -> Option<Action> {
        self.diff.map(|diff| Action::WriteToFile {
            path: self.path,
            content: self.raw,
            diff,
        })
    }
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum Error {
    #[error("Error deserializing {path}: {source}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(tauri_conf_json::deserialize),
            help(
                "knope expects the tauri.conf.json file to be an object with a top level `version` property"
            ),
            url("https://knope.tech/reference/config-file/packages/#tauri-conf")
        )
    )]
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
        "productName": "tester",
        "version": "0.1.0-rc.0"
        }"#;

        assert_eq!(
            TauriConfJson::new(RelativePathBuf::new(), content.to_string())
                .unwrap()
                .get_version(),
            &Version::from_str("0.1.0-rc.0").unwrap()
        );
    }

    #[test]
    fn test_set_version() {
        let content = r#"{
        "productName": "tester",
        "version": "0.1.0-rc.0"
        }"#;

        let new = TauriConfJson::new(RelativePathBuf::new(), content.to_string())
            .unwrap()
            .set_version(&Version::from_str("1.2.3-rc.4").unwrap())
            .unwrap()
            .write()
            .expect("diff to write");

        let expected = r#"{
  "productName": "tester",
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
        "productName": "tester",
        "version": "0.1.0-rc.0",
        "identifier": "com.knope.tester"
        }"#;

        let new = TauriConfJson::new(RelativePathBuf::new(), content.to_string())
            .unwrap()
            .set_version(&Version::from_str("1.2.3-rc.4").unwrap())
            .unwrap()
            .write()
            .expect("diff to write");

        let expected = r#"{
  "name": "tester",
  "version": "1.2.3-rc.4",
  "identifier": "com.knope.tester"
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
