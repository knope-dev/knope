use std::{fmt::Display, path::Path, str::FromStr};

use miette::Diagnostic;
use thiserror::Error;

use crate::{
    dry_run::DryRun,
    fs, git,
    git::get_current_versions_from_tags,
    releases::{semver::Version, tag_name, versioned_file::VersionFromSource, PackageName},
};

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error("No module line found in go.mod file")]
    #[diagnostic(
        code(go::no_module_line),
        help("The go.mod file does not contain a module line. This is required for the step to work."),
    )]
    MissingModuleLine,
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
    new_version: &Version,
    path: &Path,
) -> Result<String, Error> {
    let original_module_line = content
        .lines()
        .find(|line| line.starts_with("module "))
        .ok_or(Error::MissingModuleLine)?;
    let mut module_line = ModuleLine::from_str(original_module_line)?;
    module_line.major_version = Some(new_version.stable_component().major);
    module_line.version = Some(new_version.clone());

    let new_content = content.replace(original_module_line, &module_line.to_string());
    fs::write(dry_run, &new_version.to_string(), path, &new_content)?;
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
            .and_then(|_| parts.get(3))
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

    use crate::releases::go::ModuleLine;

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
    dry_run: DryRun,
) -> Result<(), git::Error> {
    let parent_dir = path.parent().map(Path::to_string_lossy);
    if let Some(parent_dir) = parent_dir {
        if !parent_dir.is_empty() {
            let tag = format!("{parent_dir}/v{version}");
            git::create_tag(dry_run, tag)?;
        }
        // If there's not a nested dir, the tag will equal the release tag, so creating it here would cause a conflict later.
    }
    Ok(())
}

/// Gets the version from the comment in the `go.mod` file, if any, or defers to the latest tag
/// for the module.
pub(crate) fn get_version(content: &str, path: &Path) -> Result<VersionFromSource, Error> {
    let prefix = path.parent().map(Path::to_string_lossy).and_then(|prefix| {
        if prefix.is_empty() {
            None
        } else {
            Some(prefix)
        }
    });
    let module_line = content
        .lines()
        .find(|line| line.starts_with("module "))
        .map(ModuleLine::from_str)
        .ok_or(Error::MissingModuleLine)??;
    if let Some(version) = module_line.version {
        return Ok(VersionFromSource {
            version,
            source: path.display().to_string(),
        });
    }

    if let Some(version_from_tag) = get_current_versions_from_tags(prefix.as_deref())
        .map(|current_versions| {
            current_versions
                .into_latest()
                .map(|version| VersionFromSource {
                    source: format!(
                        "Git tag {tag}",
                        tag = tag_name(&version, prefix.map(PackageName::from).as_ref())
                    ),
                    version,
                })
        })
        .map_err(Error::from)
        .transpose()
    {
        return version_from_tag;
    }

    Ok(VersionFromSource {
        version: Version::default(),
        source: "Default—no matching tags detected".to_string(),
    })
}
