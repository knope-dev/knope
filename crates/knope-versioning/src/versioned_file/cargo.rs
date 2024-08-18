#[cfg(feature = "miette")]
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use thiserror::Error;
use toml_edit::{value, DocumentMut, TomlError};

use crate::{semver::Version, Action};

#[derive(Clone, Debug)]
pub struct Cargo {
    path: RelativePathBuf,
    document: DocumentMut,
    diff: Vec<String>,
}

impl Cargo {
    /// Parses the raw TOML to determine the package version.
    ///
    /// # Errors
    ///
    /// If the TOML is invalid or missing a required property.
    pub fn new(path: RelativePathBuf, toml: &str) -> Result<Self, Error> {
        let document: DocumentMut = toml.parse().map_err(|source| Error::Toml {
            source,
            path: path.clone(),
        })?;
        Ok(Self {
            path,
            document,
            diff: Vec::new(),
        })
    }

    pub fn get_version(&self) -> Result<Version, Error> {
        self.document
            .get("package")
            .and_then(|package| package.get("version")?.as_str())
            .ok_or_else(|| Error::MissingRequiredProperties {
                property: "package.version",
                path: self.path.clone(),
            })?
            .parse()
            .map_err(Error::Semver)
    }

    #[must_use]
    pub(crate) fn get_path(&self) -> &RelativePathBuf {
        &self.path
    }

    #[must_use]
    pub fn set_version(mut self, new_version: &Version, dependency: Option<&str>) -> Self {
        let diff = if let Some(dependency) = dependency {
            if let Some(dep) = self
                .document
                .get_mut("dependencies")
                .and_then(|deps| deps.get_mut(&dependency))
            {
                write_version_to_dep(dep, new_version);
            }
            if let Some(dep) = self
                .document
                .get_mut("dev-dependencies")
                .and_then(|deps| deps.get_mut(&dependency))
            {
                write_version_to_dep(dep, new_version);
            }
            if let Some(dep) = self
                .document
                .get_mut("workspace")
                .and_then(|workspace| workspace.get_mut("dependencies")?.get_mut(&dependency))
            {
                write_version_to_dep(dep, new_version);
            }
            format!("{dependency}.version = {new_version}")
        } else {
            let version = self
                .document
                .get_mut("package")
                .and_then(|package| package.get_mut("version"));
            if let Some(version) = version {
                *version = value(new_version.to_string());
            }
            format!("version = {new_version}")
        };
        self.diff.push(diff);
        self
    }

    pub(crate) fn write(self) -> Option<Action> {
        if self.diff.is_empty() {
            return None;
        }
        Some(Action::WriteToFile {
            path: self.path,
            content: self.document.to_string(),
            diff: self.diff.join(", "),
        })
    }
}

#[must_use]
pub fn name_from_document(document: &DocumentMut) -> Option<&str> {
    document
        .get("package")
        .and_then(|package| package.get("name")?.as_str())
}

#[must_use]
pub fn contains_dependency(document: &DocumentMut, dependency: &str) -> bool {
    document
        .get("dependencies")
        .and_then(|deps| deps.get(dependency))
        .is_some()
        || document
            .get("dev-dependencies")
            .and_then(|deps| deps.get(dependency))
            .is_some()
        || document
            .get("workspace")
            .and_then(|workspace| workspace.get("dependencies")?.get(dependency))
            .is_some()
}

#[cfg(test)]
mod test_contains_dependency {
    use super::*;
    #[test]
    fn basic_dependency() {
        let content = r#"
        [package]
        name = "tester"
        version = "1.2.3-rc.0"
        
        [dependencies]
        knope-versioning = "0.1.0"
        "#;

        let document: DocumentMut = content.parse().expect("valid toml");
        assert!(contains_dependency(&document, "knope-versioning"));
    }

    #[test]
    fn inline_table_dependency() {
        let content = r#"
        [package]
        name = "tester"
        version = "1.2.3-rc.0"
        
        [dependencies]
        knope-versioning = { version = "0.1.0" }
        "#;

        let document: DocumentMut = content.parse().expect("valid toml");
        assert!(contains_dependency(&document, "knope-versioning"));
    }

    #[test]
    fn table_dependency() {
        let content = r#"
        [package]
        name = "tester"
        version = "1.2.3-rc.0"
        
        [dependencies.knope-versioning]
        path = "../knope-versioning"
        version = "0.1.0"
        "#;

        let document: DocumentMut = content.parse().expect("valid toml");
        assert!(contains_dependency(&document, "knope-versioning"));
    }

    #[test]
    fn dev_dependency() {
        let content = r#"
        [package]
        name = "tester"
        version = "1.2.3-rc.0"
        
        [dev-dependencies]
        knope-versioning = "0.1.0"
        "#;

        let document: DocumentMut = content.parse().expect("valid toml");
        assert!(contains_dependency(&document, "knope-versioning"));
    }

    #[test]
    fn workspace_dependency() {
        let content = r#"
        [package]
        name = "tester"
        version = "1.2.3-rc.0"
        
        [workspace.dependencies]
        knope-versioning = "0.1.0"
        "#;

        let document: DocumentMut = content.parse().expect("valid toml");
        assert!(contains_dependency(&document, "knope-versioning"));
    }
}

#[allow(clippy::indexing_slicing)]
fn write_version_to_dep(dep: &mut toml_edit::Item, version: &Version) {
    if let Some(table) = dep.as_table_mut() {
        table["version"] = value(version.to_string());
    } else if let Some(table) = dep.as_inline_table_mut() {
        table["version"] = version.to_string().into();
    } else if let Some(value) = dep.as_value_mut() {
        *value = version.to_string().into();
    }
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum Error {
    #[error("Invalid TOML in {path}: {source}")]
    #[cfg_attr(feature = "miette", diagnostic(code(knope_versioning::cargo::toml),))]
    Toml {
        path: RelativePathBuf,
        #[source]
        source: TomlError,
    },
    #[error("{path} was missing required property {property}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::cargo::missing_property),
            url("https://knope.tech/reference/config-file/packages/#cargotoml")
        )
    )]
    MissingRequiredProperties {
        path: RelativePathBuf,
        property: &'static str,
    },
    #[error("{path} does not contain dependency {dependency}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::cargo::missing_dependency),
            url("https://knope.tech/reference/config-file/packages/#cargotoml")
        )
    )]
    MissingDependency {
        path: RelativePathBuf,
        dependency: String,
    },
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    Semver(#[from] crate::semver::Error),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Toml {
    package_name: String,
    version: Version,
    version_path: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::str::FromStr;

    use pretty_assertions::assert_eq;

    use super::*;
    use crate::Action;

    #[test]
    fn set_package_version() {
        let content = r#"
        [package]
        name = "tester"
        version = "0.1.0-rc.0"
        
        [dependencies]
        knope-versioning = "0.1.0"
        "#;

        let new = Cargo::new(RelativePathBuf::from("beep/Cargo.toml"), content).unwrap();

        let new_version = "1.2.3-rc.4";
        let expected = content.replace("0.1.0-rc.0", new_version);
        let new = new.set_version(&Version::from_str(new_version).unwrap(), None);

        assert_eq!(new.document.to_string(), expected);
    }

    #[test]
    fn dependencies() {
        let content = r#"
        [package]
        name = "tester"
        version = "1.2.3-rc.0"
        
        [dependencies]
        knope-versioning = "0.1.0"
        other = {path = "../other"}
        complex-requirement = "3.*"
        complex-requirement-in-object = { version = "1.2.*" }
        
        [dev-dependencies]
        knope-versioning = {path = "../blah", version = "0.1.0" }
        
        [workspace.dependencies]
        knope-versioning = "0.1.0"
        "#;

        let new = Cargo::new(RelativePathBuf::from("beep/Cargo.toml"), content).unwrap();

        let new = new.set_version(
            &Version::from_str("0.2.0").unwrap(),
            Some("knope-versioning"),
        );
        let expected = content.replace("0.1.0", "0.2.0");
        let new = new.set_version(
            &Version::from_str("2.0.0").unwrap(),
            Some("complex-requirement-in-object"),
        );
        let expected = expected.replace("1.2.*", "2.0.0");

        let expected = Action::WriteToFile {
            path: RelativePathBuf::from("beep/Cargo.toml"),
            content: expected,
            diff: "knope-versioning.version = 2.0.0, complex-requirement-in-object.version = 2.0.0"
                .to_string(),
        };

        assert_eq!(new.write().expect("diff to write"), expected);
    }
}
