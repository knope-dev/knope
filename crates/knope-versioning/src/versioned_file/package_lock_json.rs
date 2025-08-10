use std::{fmt::Write, str::FromStr};

#[cfg(feature = "miette")]
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use serde_json::{Value, to_string_pretty};
use thiserror::Error;
use tracing::warn;

use crate::{action::Action, semver, semver::Version};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageLockJson {
    path: RelativePathBuf,
    parsed: Value,
    diff: Option<String>,
}

impl PackageLockJson {
    pub(crate) fn new(path: RelativePathBuf, content: &str) -> Result<Self, Error> {
        match serde_json::from_str(content) {
            Ok(parsed) => Ok(Self {
                path,
                parsed,
                diff: None,
            }),
            Err(err) => Err(Error::Deserialize { path, source: err }),
        }
    }

    pub(crate) fn get_version(&self) -> Result<Version, Error> {
        self.parsed
            .get("version")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::MissingVersion(self.path.clone()))
            .and_then(|val| {
                Version::from_str(val).map_err(|source| Error::InvalidVersion {
                    path: self.path.clone(),
                    source,
                })
            })
    }

    pub(crate) fn get_path(&self) -> &RelativePathBuf {
        &self.path
    }

    pub(crate) fn set_version(mut self, new_version: &Version, dependency: Option<&str>) -> Self {
        if self
            .parsed
            .get("lockfileVersion")
            .and_then(Value::as_i64)
            .is_none_or(|lock_file_version| lock_file_version != 2 && lock_file_version != 3)
        {
            warn!("package-lock.json lockfileVersion is not 2 or 3, errors may occur");
        }

        let diff = self.diff.get_or_insert_default();
        if !diff.is_empty() {
            diff.push_str(", ");
        }

        if let Some(dependency_name) = dependency {
            write!(diff, "{dependency_name} = {new_version}").ok();
            let Some(packages) = self
                .parsed
                .get_mut("packages")
                .and_then(|packages| packages.as_object_mut())
            else {
                return self;
            };

            for package in packages.values_mut() {
                let Some(package) = package.as_object_mut() else {
                    continue;
                };
                if package
                    .get("name")
                    .is_some_and(|name| name.as_str() == Some(dependency_name))
                {
                    package.insert(
                        "version".to_string(),
                        Value::String(new_version.to_string()),
                    );
                }

                // Check dependencies
                if let Some(Value::Object(deps)) = package.get_mut("dependencies") {
                    if let Some(dep_value) = deps.get_mut(dependency_name) {
                        *dep_value = Value::String(new_version.to_string());
                    }
                }

                // Check devDependencies
                if let Some(Value::Object(dev_deps)) = package.get_mut("devDependencies") {
                    if let Some(dep_value) = dev_deps.get_mut(dependency_name) {
                        *dep_value = Value::String(new_version.to_string());
                    }
                }
            }
        } else {
            self.parsed.as_object_mut().and_then(|json| {
                json.insert(
                    "version".to_string(),
                    Value::String(new_version.to_string()),
                )
            });
            self.parsed.get_mut("packages").and_then(|packages| {
                let root_package = packages.get_mut("")?;
                root_package.as_object_mut()?.insert(
                    "version".to_string(),
                    Value::String(new_version.to_string()),
                );
                Some(root_package)
            });
            write!(diff, "version = {new_version}").ok();
        }

        self
    }

    pub(crate) fn write(self) -> Option<Action> {
        self.diff.and_then(|diff| {
            Some(Action::WriteToFile {
                path: self.path,
                content: to_string_pretty(&self.parsed).ok()?,
                diff,
            })
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
            code(package_lock_json::deserialize),
            url("https://knope.tech/reference/config-file/packages/#package-lockjson")
        )
    )]
    Deserialize {
        path: RelativePathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("{0} did not have a `version` field")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(package_lock_json::missing_version),
            help("Either add a `version` field or specify a `dependency` to update"),
            url("https://knope.tech/reference/config-file/packages/#package-lockjson")
        )
    )]
    MissingVersion(RelativePathBuf),
    #[error("Error parsing {path}: {source}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(package_lock_json::invalid_version),
            help("Only plain version numbers are currently supported, e.g. `1.2.3`"),
            url("https://knope.tech/reference/config-file/packages/#package-lockjson")
        )
    )]
    InvalidVersion {
        path: RelativePathBuf,
        #[source]
        source: semver::Error,
    },
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use pretty_assertions::assert_eq;
    use serde_json::{json, to_string_pretty};

    use super::*;

    #[test]
    fn test_get_version() {
        let content = r#"{
        "name": "tester",
        "version": "0.1.0-rc.0"
        }"#;

        assert_eq!(
            PackageLockJson::new(RelativePathBuf::new(), content)
                .unwrap()
                .get_version()
                .unwrap(),
            Version::from_str("0.1.0-rc.0").unwrap()
        );
    }

    #[test]
    fn test_set_version() {
        let content = r#"{
        "name": "tester",
        "version": "0.1.0-rc.0",
        "packages": {
          "": {
            "version": "0.1.0-rc.0"
          }
        }
        }"#;

        let new = PackageLockJson::new(RelativePathBuf::new(), content)
            .unwrap()
            .set_version(&Version::from_str("1.2.3-rc.4").unwrap(), None)
            .write()
            .expect("diff to write");

        let expected = r#"{
  "name": "tester",
  "version": "1.2.3-rc.4",
  "packages": {
    "": {
      "version": "1.2.3-rc.4"
    }
  }
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
        let content = json!({
            "name": "tester",
            "version": "1.0.0",
            "packages": {
                "": {
                    "dependencies": {
                        "dependency-name": "2.0.0",
                        "some-other-dependency": "0.1.0"
                    },
                    "devDependencies": {
                        "@another/dev-dependency": "0.2.0",
                        "@dev/dependency-name": "3.0.0"
                    }
                },
                "a": {
                    "name": "dependency-name",
                    "version": "2.0.0",
                    "dependencies": {
                        "some-other-dependency": "0.1.0"
                    },
                     "devDependencies": {
                      "@another/dev-dependency": "0.2.0",
                      "@dev/dependency-name": "3.0.0"
                    }
                },
                "b": {
                    "name": "some-other-dependency",
                    "version": "0.1.0",
                    "dependencies": {
                        "dependency-name": "2.0.0"
                    },
                    "devDependencies": {
                        "@another/dev-dependency": "0.2.0",
                        "@dev/dependency-name": "3.0.0"
                    }
                },
                "c": {
                    "name": "@another/dev-dependency",
                    "version": "0.2.0",
                    "dependencies": {
                        "dependency-name": "2.0.0"
                    },
                    "devDependencies": {
                        "@dev/dependency-name": "3.0.0",
                    }
                },
                "d": {
                    "name": "@dev/dependency-name",
                    "version": "3.0.0",
                    "dependencies": {
                        "dependency-name": "2.0.0"
                    },
                    "devDependencies": {
                        "@another/dev-dependency": "0.2.0",
                    }
                }
            }
        });

        // Test updating a regular dependency
        let new = PackageLockJson::new(RelativePathBuf::new(), &content.to_string())
            .unwrap()
            .set_version(
                &Version::from_str("2.1.0").unwrap(),
                Some("dependency-name"),
            )
            .write()
            .expect("diff to write");

        let expected = to_string_pretty(&content)
            .unwrap()
            .replace("2.0.0", "2.1.0");
        let expected = Action::WriteToFile {
            path: RelativePathBuf::new(),
            content: expected,
            diff: "dependency-name = 2.1.0".to_string(),
        };
        assert_eq!(new, expected);

        // Test updating a dev dependency
        let new = PackageLockJson::new(RelativePathBuf::new(), &content.to_string())
            .unwrap()
            .set_version(
                &Version::from_str("3.1.0").unwrap(),
                Some("@dev/dependency-name"),
            )
            .write()
            .expect("diff to write");

        let expected = to_string_pretty(&content)
            .unwrap()
            .replace("3.0.0", "3.1.0");
        let expected = Action::WriteToFile {
            path: RelativePathBuf::new(),
            content: expected,
            diff: "@dev/dependency-name = 3.1.0".to_string(),
        };
        assert_eq!(new, expected);
    }
}
