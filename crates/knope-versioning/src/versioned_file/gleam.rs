#[cfg(feature = "miette")]
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use thiserror::Error;
use toml_edit::{DocumentMut, TomlError, value};

use crate::{Action, semver::Version};

#[derive(Clone, Debug)]
pub struct Gleam {
    pub(super) path: RelativePathBuf,
    pub(crate) document: DocumentMut,
    diff: Option<String>,
}

impl Gleam {
    /// Parses the raw TOML but does not check content yet.
    ///
    /// # Errors
    ///
    /// If the TOML is invalid
    pub fn new(path: RelativePathBuf, toml: &str) -> Result<Self, Error> {
        let document: DocumentMut = toml.parse().map_err(|source| Error::Toml {
            source,
            path: path.clone(),
        })?;
        Ok(Self {
            path,
            document,
            diff: None,
        })
    }

    pub(super) fn get_version(&self) -> Result<Version, Error> {
        self.document
            .get("version")
            .and_then(|version| version.as_str())
            .ok_or_else(|| Error::MissingRequiredProperties {
                property: "version",
                path: self.path.clone(),
            })?
            .parse()
            .map_err(Error::Semver)
    }

    #[must_use]
    pub(super) fn set_version(mut self, new_version: &Version) -> Self {
        let version = self.document.get_mut("version");
        if let Some(version) = version {
            *version = value(new_version.to_string());
        } else {
            self.document
                .insert("version", new_version.to_string().into());
        }

        self.diff = Some(format!("version = {new_version}"));
        self
    }

    pub(super) fn write(self) -> Option<Action> {
        self.diff.map(|diff| Action::WriteToFile {
            path: self.path,
            content: self.document.to_string(),
            diff,
        })
    }
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum Error {
    #[error("Invalid TOML in {path}: {source}")]
    #[cfg_attr(feature = "miette", diagnostic(code(knope_versioning::gleam::toml),))]
    Toml {
        path: RelativePathBuf,
        #[source]
        source: TomlError,
    },
    #[error("{path} was missing required property {property}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::gleam::missing_property),
            url("https://knope.tech/reference/config-file/packages/#gleamtoml")
        )
    )]
    MissingRequiredProperties {
        path: RelativePathBuf,
        property: &'static str,
    },
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    Semver(#[from] crate::semver::Error),
}
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::str::FromStr;

    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn set_package_version() {
        let content = r#"
        version = "0.1.0-rc.0"
        "#;

        let new = Gleam::new(RelativePathBuf::from("beep/Cargo.toml"), content).unwrap();

        let new_version = "1.2.3-rc.4";
        let expected = content.replace("0.1.0-rc.0", new_version);
        let new = new.set_version(&Version::from_str(new_version).unwrap());

        assert_eq!(new.document.to_string(), expected);
    }
}
