#[cfg(feature = "miette")]
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use serde::Deserialize;
use thiserror::Error;
use toml::Spanned;

use crate::{action::Action, semver::Version};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cargo {
    path: RelativePathBuf,
    raw_toml: String,
    parsed: Toml,
}

impl Cargo {
    /// Parses the raw TOML to determine the package version.
    ///
    /// # Errors
    ///
    /// If the TOML is invalid or missing the `package.version` property.
    pub fn new(path: RelativePathBuf, raw_toml: String) -> Result<Self, Error> {
        match toml::from_str::<Toml>(&raw_toml) {
            Ok(parsed) => Ok(Cargo {
                path,
                raw_toml,
                parsed,
            }),
            Err(err) => Err(Error::Deserialize { path, source: err }),
        }
    }

    #[must_use]
    pub fn get_version(&self) -> &Version {
        self.parsed.package.version.as_ref()
    }

    #[must_use]
    pub fn get_path(&self) -> &RelativePathBuf {
        &self.path
    }

    #[must_use]
    pub fn get_package_name(&self) -> &str {
        &self.parsed.package.name
    }

    #[must_use]
    pub fn set_version(mut self, new_version: &Version) -> Action {
        let start = self.parsed.package.version.span().start + 1;
        let end = self.parsed.package.version.span().end - 1;
        let version_str = new_version.to_string();

        self.raw_toml.replace_range(start..end, &version_str);
        Action::WriteToFile {
            path: self.path,
            content: self.raw_toml,
            diff: version_str,
        }
    }
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum Error {
    #[error("Error deserializing {path}: {source}")]
    #[cfg_attr(feature = "miette", diagnostic(
        code(cargo::deserialize),
        help("Knope expects the Cargo.toml file to have `package.version` and `package.name` properties."),
        url("https://knope.tech/reference/config-file/packages/#cargotoml")
    ))]
    Deserialize {
        path: RelativePathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    Semver(#[from] crate::semver::Error),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Toml {
    pub package: Package,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Package {
    pub name: String,
    version: Spanned<Version>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::str::FromStr;

    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_set_version() {
        let content = r#"
        [package]
        name = "tester"
        version = "0.1.0-rc.0"
        
        [dependencies]
        knope-versioning = "0.1.0"
        "#;

        let new = Cargo::new(RelativePathBuf::from("beep/boop"), String::from(content)).unwrap();

        let new_version = "1.2.3-rc.4";
        let expected = content.replace("0.1.0-rc.0", new_version);
        let expected = Action::WriteToFile {
            path: RelativePathBuf::from("beep/boop"),
            content: expected,
            diff: new_version.to_string(),
        };
        let new = new.set_version(&Version::from_str(new_version).unwrap());

        assert_eq!(new, expected);
    }
}
