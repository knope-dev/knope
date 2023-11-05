use std::{fmt::Display, path::Path, str::FromStr};

use miette::Diagnostic;
use thiserror::Error;

use super::{semver::Version, versioned_file::VersionFromSource};
use crate::{
    dry_run::DryRun,
    fs,
    integrations::git::{self, get_current_versions_from_tags},
    step::releases::versioned_file::VersionSource,
    workflow::Verbose,
};

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error("No module line found in go.mod file")]
    #[diagnostic(
        code(go::no_module_line),
        help("The go.mod file does not contain a module line. This is required for the step to work."),
        url("https://knope.tech/reference/config-file/packages/#gomod")
    )]
    MissingModuleLine,
    #[error("Will not bump Go modules to 2.0.0")]
    #[diagnostic(
        code(go::cannot_increase_major_version),
        help("Go recommends a directory-based versioning strategy for major versions above v1. See the docs for more details."),
        url("https://knope.tech/recipes/multiple-major-go-versions/"),
    )]
    BumpingToV2,
    #[error("Cannot bump major versions of directory-based go modules")]
    #[diagnostic(
        code(go::major_version_directory_based),
        help("You are using directory-based major versions—Knope cannot create a new major version directory for you. \
            Create the new directory manually and add it as a new package in knope.toml."),
        url("https://knope.tech/recipes/multiple-major-go-versions/"),
    )]
    MajorVersionDirectoryBased,
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(#[from] git::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    ModuleLine(#[from] ModuleLineError),
}

/// Sets the version in go.mod, but does not create the Git tag which _actually_ is the source
/// of truth for Go versions. That will be set by [`create_version_tag`] in the [`crate::Step::Release`].
pub(crate) fn set_version_in_file(
    dry_run: DryRun,
    content: &str,
    new_version: &VersionFromSource,
    path: &Path,
) -> Result<String, Error> {
    let original_module_line = content
        .lines()
        .find(|line| line.starts_with("module "))
        .ok_or(Error::MissingModuleLine)?;
    let mut module_line = ModuleLine::from_str(original_module_line)?;
    match (module_line.major_version, path.parent(), new_version.version.stable_component().major, &new_version.source) {
        (None, _, new_major, _) if new_major == 0 || new_major == 1 => {},  // No change
        (None, _, _, VersionSource::OverrideVersion) => {},  // Override tells us that they're aware of the risks
        (None, _, _, _) => return Err(Error::BumpingToV2),  // No major version in go.mod, but we're bumping to >1 without explicit override
        (Some(module_major), _, new_major, _) if module_major == new_major => {},  // No change
        (Some(_), None, _, _)  => {} | (Some(old_major), Some(ancestor), _, _) if ancestor.display().to_string() != format!("v{old_major}")  // Allowed to bump >1 to >1 if not using path-based directories
         => {},
        (Some(_), _, _, _) => return Err(Error::MajorVersionDirectoryBased),
    }
    module_line.major_version = Some(new_version.version.stable_component().major);
    module_line.version = Some(new_version.version.clone());

    let new_content = content.replace(original_module_line, &module_line.to_string());
    fs::write(
        dry_run,
        &new_version.version.to_string(),
        path,
        &new_content,
    )?;
    Ok(new_content)
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

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum ModuleLineError {
    #[error("missing module path")]
    #[diagnostic(
        code(go::missing_module_path),
        help("The module line in go.mod must contain a module path, usually the URI of the repository.")
    )]
    MissingModulePath,
}

#[cfg(test)]
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

pub(crate) fn create_version_tag(
    path: &Path,
    version: &Version,
    existing_tag: &str,
    dry_run: DryRun,
) -> Result<(), git::Error> {
    let tag = path
        .parent()
        .and_then(|parent| {
            let parent_str = parent.to_string_lossy();
            let major = version.stable_component().major;
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
            || format!("v{version}"),
            |prefix| format!("{prefix}/v{version}"),
        );
    if tag != existing_tag {
        git::create_tag(dry_run, &tag)?; // Avoid recreating the top-level package tag
    }
    Ok(())
}

/// Gets the version from the comment in the `go.mod` file, if any, or defers to the latest tag
/// for the module.
pub(crate) fn get_version(
    content: &str,
    path: &Path,
    verbose: Verbose,
) -> Result<VersionFromSource, Error> {
    let mut parent = path.parent();
    let module_line = content
        .lines()
        .find(|line| line.starts_with("module "))
        .map(ModuleLine::from_str)
        .ok_or(Error::MissingModuleLine)??;
    if let Some(version) = module_line.version {
        return Ok(VersionFromSource {
            version,
            source: path.into(),
        });
    }

    let major_filter = if let Some(major) = module_line.major_version {
        let major_dir = format!("v{major}");
        if parent.is_some_and(|parent| parent.ends_with(&major_dir)) {
            // Major version directories are not tag prefixes!
            parent = parent.and_then(Path::parent);
            if let Verbose::Yes = verbose {
                println!(
                    "Major version directory {major_dir} detected, only tags matching that major version will be used.",
                );
            }
        }
        Some(vec![major])
    } else {
        Some(vec![0, 1])
    };

    let prefix = match parent.map(|parent| parent.display().to_string()) {
        Some(submodule) if !submodule.is_empty() => {
            if let Verbose::Yes = verbose {
                println!(
                    "{path} is in the subdirectory {submodule}, so it will be used to filter tags.",
                    path = path.display()
                );
            }
            Some(submodule)
        }
        _ => None,
    };

    if let Verbose::Yes = verbose {
        println!(
            "No version comment in {path}, searching for relevant Git tags instead.",
            path = path.display()
        );
    }

    if let Some(version_from_tag) =
        get_current_versions_from_tags(prefix.as_deref(), major_filter.as_ref(), verbose)
            .map(|current_versions| {
                current_versions.into_latest().map(|version| {
                    let tag = if let Some(prefix) = prefix {
                        format!("{prefix}/v{version}")
                    } else {
                        format!("v{version}")
                    };
                    VersionFromSource {
                        source: VersionSource::GitTag(tag),
                        version,
                    }
                })
            })
            .map_err(Error::from)
            .transpose()
    {
        return version_from_tag;
    }

    Ok(VersionFromSource {
        version: Version::default(),
        source: VersionSource::Default,
    })
}
