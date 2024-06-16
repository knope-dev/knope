use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

#[cfg(feature = "miette")]
use miette::Diagnostic;
use relative_path::{RelativePath, RelativePathBuf};
use thiserror::Error;

use crate::{action::Action, Version};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoMod {
    path: RelativePathBuf,
    raw: String,
    module_line: ModuleLine,
    version: Version,
}

impl GoMod {
    pub(crate) fn new<S: AsRef<str>>(
        path: RelativePathBuf,
        raw: String,
        git_tags: &[S],
    ) -> Result<Self, Error> {
        let module_line = raw
            .lines()
            .find(|line| line.starts_with("module "))
            .map(ModuleLine::from_str)
            .ok_or(Error::MissingModuleLine)??;

        if let Some(comment_version) = &module_line.version {
            return Ok(Self {
                path,
                raw,
                version: comment_version.clone(),
                module_line,
            });
        }

        let mut parent = path.parent();
        let major_filter = if let Some(major) = module_line.major_version {
            let major_dir = format!("v{major}");
            if parent.is_some_and(|parent| parent.ends_with(&major_dir)) {
                // Major version directories are not tag prefixes!
                parent = parent.and_then(RelativePath::parent);
            }
            vec![major]
        } else {
            vec![0, 1]
        };
        let prefix = match parent.map(RelativePath::to_string) {
            Some(submodule) if !submodule.is_empty() => format!("{submodule}/"),
            _ => String::new(),
        };

        let Some(version_from_tag) = git_tags
            .iter()
            .filter_map(|tag| tag.as_ref().strip_prefix(&prefix)?.strip_prefix('v'))
            .find_map(|tag| {
                let version = Version::from_str(tag).ok()?;
                if major_filter.contains(&version.stable_component().major) {
                    Some(version)
                } else {
                    None
                }
            })
        else {
            return Err(Error::NoMatchingTag {
                prefix,
                major_filter,
            });
        };

        Ok(GoMod {
            path,
            raw,
            module_line,
            version: version_from_tag,
        })
    }

    pub(crate) fn get_version(&self) -> &Version {
        &self.version
    }

    pub(crate) fn get_path(&self) -> &RelativePathBuf {
        &self.path
    }

    #[allow(clippy::expect_used)]
    pub(crate) fn set_version(
        mut self,
        new_version: &Version,
        versioning: GoVersioning,
    ) -> Result<[Action; 2], SetError> {
        let original_module_line = self
            .raw
            .lines()
            .find(|line| line.starts_with("module "))
            .expect("module line was found in `new`");
        self.module_line.version = Some(new_version.clone());

        let new_major = new_version.stable_component().major;
        let module_line_needs_updating = new_major > 1
            && new_major != self.module_line.major_version.unwrap_or(0)
            && versioning != GoVersioning::IgnoreMajorRules;

        if module_line_needs_updating {
            if self.module_line.major_version.is_none() && versioning != GoVersioning::BumpMajor {
                return Err(SetError::BumpingToV2);
            }
            let using_major_version_directories =
                self.module_line.major_version.is_some_and(|old_major| {
                    self.path
                        .parent()
                        .is_some_and(|parent| parent.ends_with(format!("v{old_major}")))
                });
            if using_major_version_directories {
                return Err(SetError::MajorVersionDirectoryBased);
            }
            self.module_line.major_version = Some(new_version.stable_component().major);
        }

        let new_content = self
            .raw
            .replace(original_module_line, &self.module_line.to_string());
        let tag = self
            .path
            .parent()
            .and_then(|parent| {
                let parent_str = parent.to_string();
                let major = new_version.stable_component().major;
                let prefix = parent_str
                    .strip_suffix(&format!("v{major}"))
                    .unwrap_or(&parent_str);
                let prefix = prefix.strip_suffix('/').unwrap_or(prefix);
                if prefix.is_empty() {
                    None
                } else {
                    Some(prefix.to_string())
                }
            })
            .map_or_else(
                || format!("v{new_version}"),
                |prefix| format!("{prefix}/v{new_version}"),
            );
        Ok([
            Action::WriteToFile {
                path: self.path,
                content: new_content,
            },
            Action::AddTag { tag },
        ])
    }
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum SetError {
    #[error("Will not bump Go modules to 2.0.0")]
    #[cfg_attr(feature = "miette", diagnostic(
        code(go::cannot_increase_major_version),
        help("Go recommends a directory-based versioning strategy for major versions above v1. See the docs for more details."),
        url("https://knope.tech/recipes/multiple-major-go-versions/"),
    ))]
    BumpingToV2,
    #[error("Cannot bump major versions of directory-based go modules")]
    #[cfg_attr(feature = "miette", diagnostic(
        code(go::major_version_directory_based),
        help("You are using directory-based major versions—Knope cannot create a new major version directory for you. \
                    Create the new directory manually and add it as a new package in knope.toml."),
        url("https://knope.tech/recipes/multiple-major-go-versions/"),
    ))]
    MajorVersionDirectoryBased,
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum Error {
    #[error("No module line found in go.mod file")]
    #[cfg_attr(feature = "miette", diagnostic(
        code(go::no_module_line),
        help("The go.mod file does not contain a module line. This is required for the step to work."),
        url("https://knope.tech/reference/config-file/packages/#gomod")
    ))]
    MissingModuleLine,
    #[error(transparent)]
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    ModuleLine(#[from] ModuleLineError),
    #[error("No matching tag found for the go.mod file. Searched for a tag with the prefix {prefix} and a major version of {major_filter:?}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(go::no_matching_tag),
            help("The go.mod file must have a matching tag in the repository."),
            url("https://knope.tech/reference/config-file/packages/#gomod")
        )
    )]
    NoMatchingTag {
        prefix: String,
        major_filter: Vec<u64>,
    },
}

/// The versioning strategy for Go modules.
#[derive(Debug, Default, Eq, Clone, Copy, PartialEq)]
pub enum GoVersioning {
    /// The standard versioning strategy for Go modules:
    ///
    /// 1. Major version can't be bumped beyond 1
    /// 2. Module line must end with v{major} for major versions > 1
    #[default]
    Standard,
    /// Don't worry about the major version of the module line.
    IgnoreMajorRules,
    /// Bumping the major version of the module line is okay
    BumpMajor,
}

/// The line defining the module in go.mod, formatted like `module github.com/owner/repo/v2 // v2.1.4`.
///
/// The final component of the URI will only exist for versions >=2.0.0, and is only the major
/// component of the version.
///
/// The comment at the end is maintained by Knope and may not exist for projects which haven't yet
/// used Knope to set a new version.
///
/// More details from [the go docs](https://go.dev/doc/modules/gomod-ref#module)
#[derive(Clone, Debug, Eq, PartialEq)]
struct ModuleLine {
    module: String,
    major_version: Option<u64>,
    version: Option<Version>,
}

impl FromStr for ModuleLine {
    type Err = ModuleLineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split_whitespace().collect::<Vec<_>>();
        // parts[0] is "module"
        let mut module = (*parts.get(1).ok_or(ModuleLineError::MissingModulePath)?).to_string();
        let major_version = module
            .rsplit_once('/')
            .and_then(|(_, major)| major.strip_prefix('v'))
            .and_then(|major| major.parse::<u64>().ok());
        if major_version.is_some() {
            // We store this separately for easy incrementing and rebuilding
            module = module
                .rsplit_once('/')
                .map(|(uri, _)| uri.to_string())
                .unwrap_or(module);
        }

        let version = parts
            .get(2)
            .and_then(|comment_start| (*comment_start == "//").then_some(()))
            .and_then(|()| parts.get(3))
            .and_then(|v| v.strip_prefix('v'))
            .and_then(|v| {
                if let Ok(version) = Version::from_str(v) {
                    Some(version)
                } else {
                    None
                }
            });
        Ok(Self {
            module,
            major_version,
            version,
        })
    }
}

impl Display for ModuleLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "module {}", self.module)?;
        if let Some(major_version) = self.major_version {
            if major_version > 1 {
                write!(f, "/v{major_version}")?;
            }
        }
        if let Some(version) = &self.version {
            write!(f, " // v{version}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum ModuleLineError {
    #[error("missing module path")]
    #[cfg_attr(feature = "miette", diagnostic(
        code(go::missing_module_path),
        help("The module line in go.mod must contain a module path, usually the URI of the repository.")
    ))]
    MissingModulePath,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test_go_mod {
    use super::*;

    #[test]
    fn if_module_line_has_comment_no_tags_needed() {
        let go_mod = GoMod::new(
            RelativePathBuf::from("go.mod"),
            "module github.com/owner/repo // v2.1.4".to_string(),
            &[""],
        )
        .unwrap();
        assert_eq!(go_mod.get_version(), &Version::new(2, 1, 4, None));
    }

    #[test]
    fn get_version_from_tag() {
        let go_mod = GoMod::new(
            RelativePathBuf::from("go.mod"),
            "module github.com/owner/repo".to_string(),
            &["v1.2.3"],
        )
        .unwrap();
        assert_eq!(go_mod.get_version(), &Version::new(1, 2, 3, None));
    }

    #[test]
    fn use_v1_tags() {
        let go_mod = GoMod::new(
            RelativePathBuf::from("go.mod"),
            "module github.com/owner/repo".to_string(),
            &["v1.2.3", "v2.0.0"],
        )
        .unwrap();
        assert_eq!(go_mod.get_version(), &Version::new(1, 2, 3, None));
    }

    #[test]
    fn look_for_major_tag() {
        let go_mod = GoMod::new(
            RelativePathBuf::from("go.mod"),
            "module github.com/owner/repo/v2".to_string(),
            &["v1.2.3", "v2.0.0", "v3.0.0"],
        )
        .unwrap();
        assert_eq!(go_mod.get_version(), &Version::new(2, 0, 0, None));
    }

    #[test]
    fn tag_prefix_for_submodules() {
        let go_mod = GoMod::new(
            RelativePathBuf::from("submodule/go.mod"),
            "module github.com/owner/repo/submodule".to_string(),
            &["v1.2.3", "submodule/v0.2.0", "v1.2.4"],
        )
        .unwrap();
        assert_eq!(go_mod.get_version(), &Version::new(0, 2, 0, None));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test_module_line {
    use std::str::FromStr;

    use pretty_assertions::assert_eq;

    use super::ModuleLine;

    #[test]
    fn parse_basic() {
        let line = ModuleLine::from_str("module github.com/owner/repo").unwrap();
        assert_eq!(
            line,
            ModuleLine {
                module: "github.com/owner/repo".to_string(),
                major_version: None,
                version: None,
            }
        );
    }

    #[test]
    fn parse_with_major_version() {
        let line = ModuleLine::from_str("module github.com/owner/repo/v2").unwrap();
        assert_eq!(
            line,
            ModuleLine {
                module: "github.com/owner/repo".to_string(),
                major_version: Some(2),
                version: None,
            }
        );
    }

    #[test]
    fn parse_with_version() {
        let line = ModuleLine::from_str("module github.com/owner/repo // v2.1.4").unwrap();
        assert_eq!(
            line,
            ModuleLine {
                module: "github.com/owner/repo".to_string(),
                major_version: None,
                version: Some("2.1.4".parse().unwrap()),
            }
        );
    }

    #[test]
    fn parse_with_major_version_and_version() {
        let line = ModuleLine::from_str("module github.com/owner/repo/v2 // v3.1.4").unwrap();
        assert_eq!(
            line,
            ModuleLine {
                module: "github.com/owner/repo".to_string(),
                major_version: Some(2),
                version: Some("3.1.4".parse().unwrap()),
            }
        );
    }

    #[test]
    fn parse_with_random_comment() {
        let line = ModuleLine::from_str(
            "module github.com/owner/repo/v2 // comment that is not the thing you expect",
        )
        .unwrap();
        assert_eq!(
            line,
            ModuleLine {
                module: "github.com/owner/repo".to_string(),
                major_version: Some(2),
                version: None,
            }
        );
    }

    #[test]
    fn format_basic() {
        let line = ModuleLine {
            module: "github.com/owner/repo".to_string(),
            major_version: None,
            version: None,
        };
        assert_eq!(line.to_string(), "module github.com/owner/repo");
    }

    #[test]
    fn format_with_major_version() {
        let line = ModuleLine {
            module: "github.com/owner/repo".to_string(),
            major_version: Some(2),
            version: None,
        };
        assert_eq!(line.to_string(), "module github.com/owner/repo/v2");
    }

    #[test]
    fn format_with_version() {
        let line = ModuleLine {
            module: "github.com/owner/repo".to_string(),
            major_version: None,
            version: Some("2.1.4".parse().unwrap()),
        };
        assert_eq!(line.to_string(), "module github.com/owner/repo // v2.1.4");
    }

    #[test]
    fn format_with_major_version_and_version() {
        let line = ModuleLine {
            module: "github.com/owner/repo".to_string(),
            major_version: Some(2),
            version: Some("3.1.4".parse().unwrap()),
        };
        assert_eq!(
            line.to_string(),
            "module github.com/owner/repo/v2 // v3.1.4"
        );
    }
}
