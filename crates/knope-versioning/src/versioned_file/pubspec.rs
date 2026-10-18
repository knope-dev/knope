use std::str::FromStr;

#[cfg(feature = "miette")]
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use saphyr::{EmitError, LoadableYamlNode, Mapping, Scalar, Yaml, YamlEmitter};
use thiserror::Error;

use crate::{
    action::Action,
    semver::{self, Version},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PubSpec {
    raw: String,
    version: Version,
    path: RelativePathBuf,
    diff: Option<String>,
}

impl PubSpec {
    pub(crate) fn new(path: RelativePathBuf, content: String) -> Result<Self, Error> {
        let parsed = match Yaml::load_from_str(&content) {
            Ok(parsed) => parsed,
            Err(err) => {
                return Err(Error::Deserialize {
                    path,
                    message: err.to_string(),
                });
            }
        };
        let [doc] = parsed.as_slice() else {
            return Err(Error::Deserialize {
                path,
                message: format!("Expected a single YAML document, found {}", parsed.len()),
            });
        };
        let Some(mapping) = doc.as_mapping() else {
            return Err(Error::Deserialize {
                path,
                message: "YAML was not a mapping".into(),
            });
        };
        let Some(version) = mapping.get(&Yaml::Value(Scalar::String("version".into()))) else {
            return Err(Error::Deserialize {
                path,
                message: "YAML did not contain a top-level `version` field".into(),
            });
        };
        let Some(version) = version.as_str() else {
            return Err(Error::Deserialize {
                path,
                message: "YAML `version` field was not a string".into(),
            });
        };
        let version = Version::from_str(version)?;
        Ok(PubSpec {
            raw: content,
            version,
            path,
            diff: None,
        })
    }

    pub(crate) fn get_version(&self) -> &Version {
        &self.version
    }

    pub(crate) fn get_path(&self) -> &RelativePathBuf {
        &self.path
    }

    pub(crate) fn set_version(mut self, new_version: &Version) -> Result<Self, Error> {
        let version_line = self.raw.lines().find(|line| line.starts_with("version: "));
        self.raw = if let Some(version_line) = version_line {
            // Replace only the required bit to preserve formatting & comments (since serde_yaml doesn't preserve them)
            self.raw
                .replace(version_line, &format!("version: {new_version}"))
        } else {
            // Can't replace just the one line, resort to replacing the whole thing
            let mut yaml = Yaml::load_from_str(&self.raw).map_err(|err| Error::Deserialize {
                message: err.to_string(),
                path: self.path.clone(),
            })?;
            let mut yaml = yaml
                .pop()
                .and_then(Yaml::into_mapping)
                .unwrap_or_else(Mapping::new);
            yaml.insert(
                Yaml::Value(Scalar::String("version".into())),
                Yaml::Value(Scalar::String(new_version.to_string().into())),
            );
            let mut output = String::new();

            YamlEmitter::new(&mut output).dump(&Yaml::Mapping(yaml))?;
            output
        };
        self.diff = Some(new_version.to_string());

        Ok(self)
    }

    pub(crate) fn write(self) -> Option<Action> {
        self.diff.map(|diff| Action::WriteToFile {
            content: self.raw,
            path: self.path,
            diff,
        })
    }
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum Error {
    #[error("Error deserializing {path}: {message}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(pubspec_yaml::deserialize),
            help(
                "knope expects the pubspec.yaml file to be an object with a top level `version` property"
            ),
            url("https://knope.tech/reference/config-file/packages/#pubspecyaml")
        )
    )]
    Deserialize {
        path: RelativePathBuf,
        message: String,
    },
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    Version(#[from] semver::Error),
    #[error(transparent)]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(pubspec_yaml::serialize),
            help(
                "Knope could not update pubspec.yaml in place or seriliaze it. Please report this as a bug."
            ),
            url("https://github.com/knope-dev/knope/issues/new")
        )
    )]
    EmitError(#[from] EmitError),
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
            include_str!("../../../knope/tests/prepare_release/pubspec_yaml/in/pubspec.yaml");

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
            include_str!("../../../knope/tests/prepare_release/pubspec_yaml/in/pubspec.yaml");

        let action = PubSpec::new(RelativePathBuf::from("blah/blah"), content.to_string())
            .unwrap()
            .set_version(&Version::from_str("1.2.3-rc.4").unwrap())
            .unwrap()
            .write()
            .expect("diff to write");

        let expected_content = content.replace("1.0.0", "1.2.3-rc.4");
        let expected = Action::WriteToFile {
            path: RelativePathBuf::from("blah/blah"),
            content: expected_content,
            diff: "1.2.3-rc.4".to_string(),
        };
        assert_eq!(expected, action);
    }
}
