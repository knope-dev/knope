use std::fmt::Write;

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
    diff: Option<String>,
}

impl PackageJson {
    pub(crate) fn new(path: RelativePathBuf, content: String) -> Result<Self, Error> {
        match serde_json::from_str(&content) {
            Ok(parsed) => Ok(PackageJson {
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

    pub(crate) fn set_version(
        mut self,
        new_version: &Version,
        dependency: Option<&str>,
    ) -> serde_json::Result<Self> {
        let mut json = serde_json::from_str::<Map<String, Value>>(&self.raw)?;

        if let Some(dependency_name) = dependency {
            let mut diff_value: Option<String> = None;

            if let Some(Value::Object(deps)) = json.get_mut("dependencies") {
                if let Some(updated) =
                    Self::update_dependency_entry(deps, dependency_name, new_version)
                {
                    diff_value = Some(updated);
                }
            }

            if let Some(Value::Object(dev_deps)) = json.get_mut("devDependencies") {
                if let Some(updated) =
                    Self::update_dependency_entry(dev_deps, dependency_name, new_version)
                {
                    diff_value = Some(updated);
                }
            }

            if let Some(value) = diff_value {
                self.raw = serde_json::to_string_pretty(&json)?;
                let diff = self.diff.get_or_insert_default();
                if !diff.is_empty() {
                    diff.push_str(", ");
                }
                write!(diff, "{dependency_name} = {value}").ok();
            }
        } else {
            json.insert(
                "version".to_string(),
                Value::String(new_version.to_string()),
            );
            self.raw = serde_json::to_string_pretty(&json)?;

            let diff = self.diff.get_or_insert_default();
            if !diff.is_empty() {
                diff.push_str(", ");
            }
            write!(diff, "version = {new_version}").ok();
            self.parsed.version = new_version.clone();
        }

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
            code(package_json::deserialize),
            help(
                "knope expects the package.json file to be an object with a top level `version` property"
            ),
            url("https://knope.tech/reference/config-file/packages/#packagejson")
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

impl PackageJson {
    fn update_dependency_entry(
        deps: &mut Map<String, Value>,
        dependency_name: &str,
        new_version: &Version,
    ) -> Option<String> {
        let dep_value = deps.get_mut(dependency_name)?;
        let existing = dep_value.as_str()?;

        let new_value =
            if let Some(updated) = Self::update_workspace_specifier(existing, new_version) {
                updated
            } else if existing.starts_with("workspace:") {
                existing.to_string()
            } else {
                new_version.to_string()
            };

        if new_value == existing {
            return None;
        }

        *dep_value = Value::String(new_value.clone());
        Some(new_value)
    }

    fn update_workspace_specifier(existing: &str, new_version: &Version) -> Option<String> {
        const PREFIX: &str = "workspace:";
        if !existing.starts_with(PREFIX) {
            return None;
        }

        let remainder = &existing[PREFIX.len()..];
        let digits_index = remainder.find(|c: char| c.is_ascii_digit())?;
        let (specifier_prefix, _) = remainder.split_at(digits_index);
        Some(format!("{PREFIX}{specifier_prefix}{new_version}"))
    }
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
            .set_version(&Version::from_str("1.2.3-rc.4").unwrap(), None)
            .unwrap()
            .write()
            .expect("diff to write");

        let expected = r#"{
  "name": "tester",
  "version": "1.2.3-rc.4"
}"#
        .to_string();
        let expected = Action::WriteToFile {
            path: RelativePathBuf::new(),
            content: expected,
            diff: "version = 1.2.3-rc.4".to_string(),
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
            .set_version(&Version::from_str("1.2.3-rc.4").unwrap(), None)
            .unwrap()
            .write()
            .expect("diff to write");

        let expected = r#"{
  "name": "tester",
  "version": "1.2.3-rc.4",
  "dependencies": {}
}"#
        .to_string();
        let expected = Action::WriteToFile {
            path: RelativePathBuf::new(),
            content: expected,
            diff: "version = 1.2.3-rc.4".to_string(),
        };
        assert_eq!(new, expected);
    }

    #[test]
    fn update_dependency() {
        let content = r#"{
            "name": "tester",
            "version": "1.0.0",
            "dependencies": {
                "dependency-name": "2.0.0",
                "some-other-dependency": "0.1.0"
            },
            "devDependencies": {
                "@another/dev-dependency": "0.2.0",
                "@dev/dependency-name": "3.0.0"
            }
        }"#;

        // Test updating a regular dependency
        let new = PackageJson::new(RelativePathBuf::new(), content.to_string())
            .unwrap()
            .set_version(
                &Version::from_str("2.1.0").unwrap(),
                Some("dependency-name"),
            )
            .unwrap()
            .write()
            .expect("diff to write");

        let expected = r#"{
  "name": "tester",
  "version": "1.0.0",
  "dependencies": {
    "dependency-name": "2.1.0",
    "some-other-dependency": "0.1.0"
  },
  "devDependencies": {
    "@another/dev-dependency": "0.2.0",
    "@dev/dependency-name": "3.0.0"
  }
}"#
        .to_string();
        let expected = Action::WriteToFile {
            path: RelativePathBuf::new(),
            content: expected,
            diff: "dependency-name = 2.1.0".to_string(),
        };
        assert_eq!(new, expected);

        // Test updating a dev dependency
        let new = PackageJson::new(RelativePathBuf::new(), content.to_string())
            .unwrap()
            .set_version(
                &Version::from_str("3.1.0").unwrap(),
                Some("@dev/dependency-name"),
            )
            .unwrap()
            .write()
            .expect("diff to write");

        let expected = r#"{
  "name": "tester",
  "version": "1.0.0",
  "dependencies": {
    "dependency-name": "2.0.0",
    "some-other-dependency": "0.1.0"
  },
  "devDependencies": {
    "@another/dev-dependency": "0.2.0",
    "@dev/dependency-name": "3.1.0"
  }
}"#
        .to_string();
        let expected = Action::WriteToFile {
            path: RelativePathBuf::new(),
            content: expected,
            diff: "@dev/dependency-name = 3.1.0".to_string(),
        };
        assert_eq!(new, expected);
    }

    #[test]
    fn update_workspace_dependency() {
        let content = r#"{
            "name": "tester",
            "version": "1.0.0",
            "dependencies": {
                "@scope/dependency": "workspace:^1.0.0"
            }
        }"#;

        let new = PackageJson::new(RelativePathBuf::new(), content.to_string())
            .unwrap()
            .set_version(
                &Version::from_str("2.0.0").unwrap(),
                Some("@scope/dependency"),
            )
            .unwrap()
            .write()
            .expect("diff to write");

        let expected = r#"{
  "name": "tester",
  "version": "1.0.0",
  "dependencies": {
    "@scope/dependency": "workspace:^2.0.0"
  }
}"#
        .to_string();
        let expected = Action::WriteToFile {
            path: RelativePathBuf::new(),
            content: expected,
            diff: "@scope/dependency = workspace:^2.0.0".to_string(),
        };
        assert_eq!(new, expected);
    }
}
