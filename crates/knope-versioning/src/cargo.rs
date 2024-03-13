#[cfg(feature = "miette")]
use miette::Diagnostic;
use serde::Deserialize;
use thiserror::Error;
use toml::Spanned;

use crate::Version;

#[derive(Debug)]
pub struct Cargo {
    #[allow(dead_code)]
    path: String,
    raw_toml: String,
    parsed: Toml,
}

impl Cargo {
    /// Parses the raw TOML to determine the package version.
    ///
    /// # Errors
    ///
    /// If the TOML is invalid or missing the `package.version` property.
    pub fn new(path: String, raw_toml: String) -> Result<Self, Error> {
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
    pub fn get_toml(&self) -> &str {
        &self.raw_toml
    }

    #[must_use]
    pub fn set_version(mut self, new_version: Version) -> Self {
        // Account for quotes with +- 1
        let start = self.parsed.package.version.span().start + 1;
        let end = self.parsed.package.version.span().end - 1;
        let version_str = new_version.to_string();
        *self.parsed.package.version.as_mut() = new_version;

        self.raw_toml.replace_range(start..end, &version_str);
        self
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
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    Semver(#[from] crate::semver::Error),
}

#[derive(Debug, Deserialize)]
pub struct Toml {
    pub package: Package,
}

#[derive(Debug, Deserialize)]
pub struct Package {
    pub name: String,
    version: Spanned<Version>,
}

#[cfg(test)]
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

        let new = Cargo::new(String::from("beep/boop"), String::from(content)).unwrap();

        let new_version = "1.2.3-rc.4";
        let expected = content.replace("0.1.0-rc.0", new_version);
        let new = new.set_version(Version::from_str(new_version).unwrap());

        assert_eq!(new.get_toml(), expected);
    }
}
