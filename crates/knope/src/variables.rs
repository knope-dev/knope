use indexmap::IndexMap;
use knope_versioning::Version;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};

use crate::{
    integrations::git::branch_name_from_issue,
    state,
    state::State,
    step::releases::{package, semver, Package, Release},
    workflow::Verbose,
};

/// Describes a value that can replace an arbitrary string in certain steps.
///
/// <https://knope.tech/reference/config-file/variables//>
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(crate) enum Variable {
    /// The version of the package, if only a single package is configured (error if multiple).
    Version,
    /// The generated branch name for the selected issue. Note that this means the workflow must
    /// already be in [`State::IssueSelected`] when this variable is used.
    IssueBranch,
    /// Get the current changelog entry from the latest release.
    ChangelogEntry,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// A template string and the variables that should be replaced in it.
pub(crate) struct Template {
    pub(crate) template: String,
    #[serde(default)]
    pub(crate) variables: IndexMap<String, Variable>,
}

/// Replace declared variables in the string and return the new string.
pub(crate) fn replace_variables(template: Template, state: &State) -> Result<String, Error> {
    let mut version_cache = None;
    let mut package_cache = None;
    let Template {
        mut template,
        variables,
    } = template;
    for (var_name, var_type) in variables {
        match var_type {
            Variable::Version => {
                let version = if let Some(version) = version_cache.take() {
                    version
                } else {
                    let package = if let Some(package) = package_cache.take() {
                        package
                    } else {
                        first_package(state)?
                    };
                    package_cache = Some(package);
                    latest_version(state.verbose, package, &state.all_git_tags)?
                };
                template = template.replace(&var_name, &version.to_string());
                version_cache = Some(version);
            }
            Variable::ChangelogEntry => {
                let package = if let Some(package) = package_cache.take() {
                    package
                } else {
                    first_package(state)?
                };
                package_cache = Some(package);
                let version = if let Some(version) = version_cache.take() {
                    version
                } else {
                    latest_version(state.verbose, package, state.all_git_tags.as_ref())?
                };
                let changelog_entry = package
                    .prepared_release
                    .as_ref()
                    .and_then(Release::body)
                    .map_or_else(
                        || {
                            package
                                .changelog
                                .as_ref()
                                .and_then(|changelog| {
                                    changelog
                                        .get_release(
                                            &version,
                                            package.files.clone(),
                                            package.go_versioning,
                                        )
                                        .transpose()
                                })
                                .transpose()?
                                .and_then(|release| release.body())
                                .ok_or_else(|| Error::NoChangelogEntry(version.clone()))
                        },
                        Ok,
                    )?;
                template = template.replace(&var_name, &changelog_entry);
                version_cache = Some(version);
            }
            Variable::IssueBranch => match &state.issue {
                state::Issue::Initial => return Err(Error::NoIssueSelected),
                state::Issue::Selected(issue) => {
                    template = template.replace(&var_name, &branch_name_from_issue(issue));
                }
            },
        }
    }
    Ok(template)
}

fn latest_version(
    verbose: Verbose,
    package: &Package,
    git_tags: &[String],
) -> Result<Version, Error> {
    Ok(if let Some(release) = package.prepared_release.as_ref() {
        release.version.clone()
    } else {
        package
            .get_version(verbose, git_tags)
            .into_latest()
            .ok_or(Error::NoCurrentVersion)?
    })
}

fn first_package(state: &State) -> Result<&Package, Error> {
    if state.packages.len() > 1 {
        Err(Error::TooManyPackages)
    } else if let Some(package) = state.packages.first() {
        Ok(package)
    } else {
        Err(package::Error::NoDefinedPackages.into())
    }
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("Too many packages defined")]
    #[diagnostic(
        code(variables::too_many_packages),
        help("The Version and Changelog variables can only be used with a single [package].")
    )]
    TooManyPackages,
    #[error(transparent)]
    #[diagnostic(transparent)]
    Package(#[from] package::Error),
    #[error("Could not determine the current version of the package")]
    #[diagnostic(
        code(variables::no_current_version),
        url("https://knope.tech/reference/concepts/package#version")
    )]
    NoCurrentVersion,
    #[error("Could not find a changelog entry for version {0}")]
    #[diagnostic(
        code(variables::no_changelog_entry),
        url("https://knope.tech/reference/concepts/changelog/#versions")
    )]
    NoChangelogEntry(Version),
    #[error("No issue selected")]
    #[diagnostic(
        code(variables::no_issue_selected),
        help("The IssueBranch command variable requires selecting an issue first with SelectGitHubIssue or SelectJiraIssue")
    )]
    NoIssueSelected,
    #[error(transparent)]
    #[diagnostic(transparent)]
    SemVer(#[from] semver::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    ChangelogParse(#[from] crate::step::releases::changelog::ParseError),
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::indexing_slicing)]
mod test_replace_variables {
    use std::fs::write;

    use knope_versioning::{Version, VersionedFile, VersionedFilePath};
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    use super::*;
    use crate::{
        state,
        step::{
            issues::Issue,
            releases::{
                changelog::HeaderLevel, conventional_commits::ConventionalCommit,
                package::ChangelogSections, Change, ChangeType, Package, Release,
            },
        },
        workflow::Verbose,
    };

    fn package() -> (Package, TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let changelog = temp_dir.path().join("CHANGELOG.md");
        write(&changelog, "").unwrap();

        (
            Package {
                files: knope_versioning::Package::new(vec![VersionedFile::new(
                    &VersionedFilePath::new("Cargo.toml".into()).unwrap(),
                    "[package]\nversion = \"1.2.3\"\nname=\"blah\"".into(),
                    &[""],
                )
                .unwrap()])
                .ok(),
                changelog: Some(changelog.try_into().unwrap()),
                ..Package::default()
            },
            temp_dir,
        )
    }

    #[test]
    fn replace_prepared_version() {
        let template = "blah $$ other blah".to_string();
        let mut variables = IndexMap::new();
        variables.insert("$$".to_string(), Variable::Version);
        let mut state = State::new(None, None, None, vec![package().0], Vec::new(), Verbose::No);
        let version = Version::new(1, 2, 3, None);
        state.packages[0].prepared_release = Some(Release::empty(version.clone(), Vec::new()));

        let result = replace_variables(
            Template {
                template,
                variables,
            },
            &state,
        )
        .unwrap();

        assert_eq!(result, format!("blah {version} other blah"));
    }

    #[test]
    fn replace_issue_branch() {
        let template = "blah $$ other blah".to_string();
        let mut variables = IndexMap::new();
        variables.insert("$$".to_string(), Variable::IssueBranch);
        let issue = Issue {
            key: "13".to_string(),
            summary: "1234".to_string(),
        };
        let expected_branch_name = branch_name_from_issue(&issue);
        let state = State {
            jira_config: None,
            github: state::GitHub::New,
            github_config: None,
            gitea: state::Gitea::New,
            gitea_config: None,
            issue: state::Issue::Selected(issue),
            packages: Vec::new(),
            all_git_tags: Vec::new(),
            verbose: Verbose::No,
        };

        let result = replace_variables(
            Template {
                template,
                variables,
            },
            &state,
        )
        .unwrap();

        assert_eq!(result, format!("blah {expected_branch_name} other blah"));
    }

    #[test]
    fn replace_changelog_entry_prepared_release() {
        let template = "blah $$ other blah".to_string();
        let mut variables = IndexMap::new();
        variables.insert("$$".to_string(), Variable::ChangelogEntry);
        let mut state = State::new(None, None, None, vec![package().0], Vec::new(), Verbose::No);
        let version = Version::new(1, 2, 3, None);
        let changes = [Change::ConventionalCommit(ConventionalCommit {
            change_type: ChangeType::Feature,
            message: "Blah".to_string(),
            original_source: String::new(),
        })];
        let changelog_sections = ChangelogSections::default();
        state.packages[0].prepared_release = Some(Release::new(
            version.clone(),
            &changes,
            &changelog_sections,
            HeaderLevel::H2,
            Vec::new(),
        ));

        let result = replace_variables(
            Template {
                template,
                variables,
            },
            &state,
        )
        .unwrap();

        let changelog_entry = state.packages[0]
            .prepared_release
            .as_ref()
            .unwrap()
            .body()
            .unwrap();
        assert_eq!(result, format!("blah {changelog_entry} other blah"));
    }
}
