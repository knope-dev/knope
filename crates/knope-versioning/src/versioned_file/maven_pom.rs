use std::io::BufWriter;

#[cfg(feature = "miette")]
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use thiserror::Error;
use xml::writer::Error as EmitterError;
use xmltree::{Element, EmitterConfig, XMLNode};

use crate::{Action, semver::Version};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MavenPom {
    pub(crate) path: RelativePathBuf,
    raw: String,
    project: Element,
    diff: Option<String>,
}

impl MavenPom {
    pub(crate) fn new(path: RelativePathBuf, content: String) -> Result<Self, Error> {
        let element = match Element::parse(content.as_bytes()) {
            Ok(element) => element,
            Err(err) => return Err(Error::Xml { path, source: err }),
        };

        if element.name != "project" {
            return Err(Error::MissingRequiredProperties {
                path,
                property: "project",
            });
        }

        Ok(MavenPom {
            path,
            raw: content,
            project: element,
            diff: None,
        })
    }

    pub(crate) fn get_version(&self) -> Result<Version, Error> {
        let version = self
            .project
            .get_child("version")
            .and_then(xmltree::Element::get_text)
            .ok_or_else(|| Error::MissingRequiredProperties {
                property: "project.version",
                path: self.path.clone(),
            })?;
        version.parse().map_err(Error::Semver)
    }

    pub(crate) fn set_version(mut self, new_version: &Version) -> Result<Self, Error> {
        let version_node = if let Some(version_node) = self.project.get_mut_child("version") {
            version_node
        } else {
            let version_node = XMLNode::Element(Element::new("version"));

            // Attempt to insert after artifactId, otherwise at the end
            let position = self
                .project
                .children
                .iter()
                .position(|child| {
                    if let Some(child) = child.as_element() {
                        child.name == "artifactId"
                    } else {
                        false
                    }
                })
                .map_or(self.project.children.len(), |index| index + 1);
            self.project.children.insert(position, version_node);

            #[allow(clippy::unwrap_used)] // we just inserted the element
            self.project
                .children
                .get_mut(position)
                .and_then(XMLNode::as_mut_element)
                .unwrap()
        };

        version_node.children = vec![XMLNode::Text(new_version.to_string())];
        self.diff = Some(format!("project.version = {new_version}"));
        self.raw = self.to_string()?;
        Ok(self)
    }

    pub(crate) fn write(self) -> Option<Action> {
        self.diff.map(|diff| Action::WriteToFile {
            path: self.path,
            content: self.raw,
            diff,
        })
    }

    fn to_string(&self) -> Result<String, Error> {
        // Formatting is a bit awkward here.
        // We would like to patch the file in place and preserve the original formatting
        // as much as possible.
        let mut buf = BufWriter::new(Vec::new());
        self.project
            .write_with_config(
                &mut buf,
                EmitterConfig {
                    write_document_declaration: false,
                    perform_indent: true,
                    ..Default::default()
                },
            )
            .map_err(|err| Error::Serialize {
                path: self.path.clone(),
                source: err,
            })?;

        #[allow(clippy::unwrap_used)] // serializer writes valid utf-8
        Ok(String::from_utf8(buf.into_inner().unwrap()).unwrap())
    }
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum Error {
    #[error("Invalid XML in {path}: {source}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::maven_pom::xml),
            help("knope expects the pom.xml file to be maven project with a version property"),
            url("https://knope.tech/reference/config-file/packages/#pomxml")
        )
    )]
    Xml {
        path: RelativePathBuf,
        #[source]
        source: xmltree::ParseError,
    },

    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::maven_pom::serialize),
            help("an internal error prevented knope from writing the new version to the file"),
            url("https://knope.tech/reference/config-file/packages/#pomxml")
        )
    )]
    #[error("Failed to serialize XML to {path}: {source}")]
    Serialize {
        path: RelativePathBuf,

        #[source]
        source: EmitterError,
    },

    #[error("{path} was missing required property {property}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::maven_pom::missing_property),
            url("https://knope.tech/reference/config-file/packages/#pomxml")
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
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn get_version() {
        let content = r"
        <project>
          <modelVersion>4.0.0</modelVersion>
          <groupId>com.mycompany.app</groupId>
          <artifactId>my-app</artifactId>
          <version>1.2.3-rc.1</version>
        </project>
        ";

        assert_eq!(
            MavenPom::new(RelativePathBuf::new(), content.to_string())
                .unwrap()
                .get_version()
                .unwrap(),
            Version::from_str("1.2.3-rc.1").unwrap()
        );
    }

    #[test]
    fn error_on_missing_project() {
        let content = r"
        <somethingElse>
          Invalid data  
        </somethingElse>
        ";

        let pom = MavenPom::new(RelativePathBuf::new(), content.to_string());
        if let Err(Error::MissingRequiredProperties { property, .. }) = pom {
            assert_eq!(property, "project");
        } else {
            panic!("Expected error");
        }
    }

    #[test]
    fn error_on_missing_version() {
        let content = r"
        <project>
          <modelVersion>4.0.0</modelVersion>
          <groupId>com.mycompany.app</groupId>
          <artifactId>my-app</artifactId>
        </project>
        ";

        let pom = MavenPom::new(RelativePathBuf::new(), content.to_string()).unwrap();
        if let Err(Error::MissingRequiredProperties { property, .. }) = pom.get_version() {
            assert_eq!(property, "project.version");
        } else {
            panic!("Expected error");
        }
    }

    #[test]
    fn set_version() {
        let content = r"
        <project>
          <modelVersion>4.0.0</modelVersion>
          <groupId>com.mycompany.app</groupId>
          <artifactId>my-app</artifactId>
          <version>0.1.0-rc.0</version>
        </project>
        ";

        let pom = MavenPom::new(RelativePathBuf::new(), content.to_string()).unwrap();

        let new_version = Version::from_str("1.2.3-rc.4").unwrap();
        let pom = pom.set_version(&new_version).unwrap();
        let expected = r"<project>
  <modelVersion>4.0.0</modelVersion>
  <groupId>com.mycompany.app</groupId>
  <artifactId>my-app</artifactId>
  <version>1.2.3-rc.4</version>
</project>";
        assert_eq!(pom.raw, expected);
    }
}
