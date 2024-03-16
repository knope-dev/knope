#[cfg(feature = "miette")]
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use serde_yaml::{from_str, to_string, Mapping, Value};
use thiserror::Error;

use crate::{action::Action, semver, Version};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PubSpec {
    raw: String,
    parsed: Yaml,
    path: RelativePathBuf,
}

impl PubSpec {
    pub(crate) fn new(path: RelativePathBuf, content: String) -> Result<Self, Error> {
        match from_str(&content) {
            Ok(parsed) => Ok(PubSpec {
                raw: content,
                parsed,
                path,
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

    pub(crate) fn set_version(self, new_version: &Version) -> serde_yaml::Result<Action> {
        let version_line = self.raw.lines().find(|line| line.starts_with("version: "));
        let new_content = if let Some(version_line) = version_line {
            // Replace only the required bit to preserve formatting & comments (since serde_yaml doesn't preserve them)
            self.raw.replace(
                version_line,
                to_string(&Yaml {
                    version: new_version.clone(),
                })?
                .trim(),
            )
        } else {
            // Can't replace just the one line, resort to replacing the whole thing
            let mut yaml = from_str::<Mapping>(&self.raw)?;
            yaml.insert(
                Value::String("version".to_string()),
                Value::String(new_version.to_string()),
            );
            to_string(&yaml)?
        };

        Ok(Action::WriteToFile {
            path: self.path,
            content: new_content,
        })
    }
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum Error {
    #[error("Error deserializing {path}: {source}")]
    #[cfg_attr(feature = "miette", diagnostic(
        code(pubspec_yaml::deserialize),
        help("knope expects the pubspec.yaml file to be an object with a top level `version` property"),
        url("https://knope.tech/reference/config-file/packages/#pubspecyaml")
    ))]
    Deserialize {
        path: RelativePathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    Version(#[from] semver::Error),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Yaml {
    version: Version,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::str::FromStr;

    use pretty_assertions::assert_eq;

    use super::*;
    #[test]
    fn test_get_version() {
        let content =
            include_str!("../../knope/tests/prepare_release/pubspec_yaml/in/pubspec.yaml");

        assert_eq!(
            PubSpec::new(RelativePathBuf::new(), content.to_string())
                .unwrap()
                .get_version(),
            &Version::from_str("1.0.0").unwrap()
        );
    }

    #[test]
    fn test_set_version() {
        let content =
            include_str!("../../knope/tests/prepare_release/pubspec_yaml/in/pubspec.yaml");

        let action = PubSpec::new(RelativePathBuf::from("blah/blah"), content.to_string())
            .unwrap()
            .set_version(&Version::from_str("1.2.3-rc.4").unwrap())
            .unwrap();

        let expected_content = content.replace("1.0.0", "1.2.3-rc.4");
        let expected = Action::WriteToFile {
            path: RelativePathBuf::from("blah/blah"),
            content: expected_content,
        };
        assert_eq!(expected, action);
    }
}
