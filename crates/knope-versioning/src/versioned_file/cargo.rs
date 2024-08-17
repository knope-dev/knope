#[cfg(feature = "miette")]
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use thiserror::Error;
use toml_edit::{value, DocumentMut, TomlError};

use super::Path;
use crate::{action::Action, semver::Version};

#[derive(Clone, Debug)]
pub struct Cargo {
    path: Path,
    document: DocumentMut,
    package_name: String,
    version: Version,
}

impl Cargo {
    /// Parses the raw TOML to determine the package version.
    ///
    /// # Errors
    ///
    /// If the TOML is invalid or missing a required property.
    pub fn new(path: Path, toml: &str) -> Result<Self, Error> {
        let document: DocumentMut = toml.parse().map_err(|source| Error::Toml {
            source,
            path: path.as_path(),
        })?;
        let package_name = name_from_document(&document)
            .ok_or_else(|| Error::MissingRequiredProperties {
                property: "package.name",
                path: path.as_path(),
            })?
            .to_string();
        let version = if let Some(dependency) = path.dependency.as_ref() {
            dependency_from_document(&path, &document, dependency)?
        } else {
            document
                .get("package")
                .and_then(|package| package.get("version")?.as_str())
                .ok_or_else(|| Error::MissingRequiredProperties {
                    property: "package.version",
                    path: path.as_path(),
                })?
                .parse()?
        };
        Ok(Self {
            path,
            document,
            package_name,
            version,
        })
    }

    #[must_use]
    pub fn get_version(&self) -> &Version {
        &self.version
    }

    #[must_use]
    pub(crate) fn get_path(&self) -> RelativePathBuf {
        self.path.as_path()
    }

    #[must_use]
    pub fn get_package_name(&self) -> &str {
        &self.package_name
    }

    #[must_use]
    pub fn set_version(mut self, new_version: &Version) -> Action {
        let path = self.path.as_path();
        let diff = if let Some(dependency) = self.path.dependency {
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
            format!("{dependency}@{new_version}")
        } else {
            let version = self
                .document
                .get_mut("package")
                .and_then(|package| package.get_mut("version"));
            if let Some(version) = version {
                *version = value(new_version.to_string());
            }
            new_version.to_string()
        };
        Action::WriteToFile {
            path,
            content: self.document.to_string(),
            diff,
        }
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

fn dependency_from_document(
    path: &Path,
    document: &DocumentMut,
    dependency: &str,
) -> Result<Version, Error> {
    let item = document
        .get("dependencies")
        .and_then(|deps| deps.get(dependency))
        .or_else(|| {
            document
                .get("dev-dependencies")
                .and_then(|deps| deps.get(dependency))
        })
        .or_else(|| {
            document
                .get("workspace")
                .and_then(|workspace| workspace.get("dependencies")?.get(dependency))
        })
        .ok_or_else(|| Error::MissingDependency {
            path: path.as_path(),
            dependency: dependency.to_string(),
        })?;

    if let Some(version) = item.as_str() {
        version.parse().map_err(Error::Semver)
    } else {
        item.get("version")
            .and_then(|version| version.as_str())
            .ok_or_else(|| Error::MissingDependency {
                // TODO: specify error for version not set
                path: path.as_path(),
                dependency: dependency.to_string(),
            })
            .and_then(|version| version.parse().map_err(Error::Semver))
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

    #[test]
    fn set_package_version() {
        let content = r#"
        [package]
        name = "tester"
        version = "0.1.0-rc.0"
        
        [dependencies]
        knope-versioning = "0.1.0"
        "#;

        let new = Cargo::new(Path::new("beep/Cargo.toml".into(), None).unwrap(), content).unwrap();

        let new_version = "1.2.3-rc.4";
        let expected = content.replace("0.1.0-rc.0", new_version);
        let expected = Action::WriteToFile {
            path: RelativePathBuf::from("beep/Cargo.toml"),
            content: expected,
            diff: new_version.to_string(),
        };
        let new = new.set_version(&Version::from_str(new_version).unwrap());

        assert_eq!(new, expected);
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

        let new = Cargo::new(
            Path::new(
                "beep/Cargo.toml".into(),
                Some("knope-versioning".to_string()),
            )
            .unwrap(),
            content,
        )
        .unwrap();

        let new_version = "0.2.0";
        let expected = content.replace("0.1.0", new_version);
        let expected = Action::WriteToFile {
            path: RelativePathBuf::from("beep/Cargo.toml"),
            content: expected,
            diff: format!("knope-versioning@{new_version}"),
        };
        let new = new.set_version(&Version::from_str(new_version).unwrap());

        assert_eq!(new, expected);
    }
}
