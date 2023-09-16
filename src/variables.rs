use indexmap::IndexMap;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};

use crate::{
    git::branch_name_from_issue,
    state,
    state::State,
    step::releases::{package, semver, semver::Version, Package},
    workflow::Verbose,
};

/// Describes a value that can replace an arbitrary string in certain steps.
///
/// <https://knope-dev.github.io/knope/config/variables.html/>
#[derive(Debug, Deserialize, Serialize)]
pub(crate) enum Variable {
    /// The version of the package, if only a single package is configured (error if multiple).
    Version,
    /// The generated branch name for the selected issue. Note that this means the workflow must
    /// already be in [`State::IssueSelected`] when this variable is used.
    IssueBranch,
    /// Get the current changelog entry from the latest release.
    ChangelogEntry,
}

/// Replace declared variables in the string and return the new string.
pub(crate) fn replace_variables(
    mut command: String,
    variables: IndexMap<String, Variable>,
    state: &State,
    verbose: Verbose,
) -> Result<String, Error> {
    let mut version_cache = None;
    let mut package_cache = None;
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
                    latest_version(verbose, package)?
                };
                command = command.replace(&var_name, &version.to_string());
                version_cache = Some(version);
            }
            Variable::ChangelogEntry => {
                let package = if let Some(package) = package_cache.take() {
                    package
                } else {
                    first_package(state)?
                };
                let version = if let Some(version) = version_cache.take() {
                    version
                } else {
                    latest_version(verbose, package)?
                };
                let changelog_entry = package
                    .prepared_release
                    .as_ref()
                    .and_then(|prepared_release| prepared_release.new_changelog.clone())
                    .map_or_else(
                        || {
                            package
                                .changelog
                                .as_ref()
                                .and_then(|changelog| changelog.get_section(&version))
                                .ok_or_else(|| Error::NoChangelogEntry(version.clone()))
                        },
                        Ok,
                    )?;
                command = command.replace(&var_name, &changelog_entry);
                version_cache = Some(version);
            }
            Variable::IssueBranch => match &state.issue {
                state::Issue::Initial => return Err(Error::NoIssueSelected),
                state::Issue::Selected(issue) => {
                    command = command.replace(&var_name, &branch_name_from_issue(issue));
                }
            },
        }
    }
    Ok(command)
}

fn latest_version(verbose: Verbose, package: &Package) -> Result<Version, Error> {
    Ok(if let Some(release) = package.prepared_release.as_ref() {
        release.new_version.clone()
    } else {
        package
            .get_version(verbose)?
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
        Err(package::Error::no_defined_packages_with_help().into())
    }
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("Too many packages defined")]
    #[diagnostic(
        code(variables::too_many_packages),
        help("The Version variable in a Command step can only be used with a single [package].")
    )]
    TooManyPackages,
    #[error(transparent)]
    #[diagnostic(transparent)]
    Package(#[from] package::Error),
    #[error("Could not determine the current version of the package")]
    #[diagnostic(
        code(variables::no_current_version),
        url("https://knope-dev.github.io/knope/config/variables.html#version")
    )]
    NoCurrentVersion,
    #[error("Could not find a changelog entry for version {0}")]
    #[diagnostic(
        code(variables::no_changelog_entry),
        url("https://knope-dev.github.io/knope/config/variables.html#changelogentry")
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
}

#[cfg(test)]
mod test_replace_variables {
    use std::fs::write;

    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    use super::*;
    use crate::{
        state,
        step::{
            issues::Issue,
            releases::{semver::Version, Package, Release},
        },
        workflow::Verbose,
    };

    fn package() -> (Package, TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        write(&cargo_toml, "[package]\nversion = \"1.2.3\"").unwrap();
        let changelog = temp_dir.path().join("CHANGELOG.md");
        write(&changelog, "").unwrap();

        (
            Package {
                versioned_files: vec![cargo_toml.try_into().unwrap()],
                changelog: Some(changelog.try_into().unwrap()),
                ..Package::default()
            },
            temp_dir,
        )
    }

    #[test]
    fn multiple_variables() {
        let command = "blah $$ branch_name".to_string();
        let mut variables = IndexMap::new();
        variables.insert("$$".to_string(), Variable::Version);
        variables.insert("branch_name".to_string(), Variable::IssueBranch);
        let issue = Issue {
            key: "13".to_string(),
            summary: "1234".to_string(),
        };
        let expected_branch_name = branch_name_from_issue(&issue);
        let state = State {
            jira_config: None,
            github: state::GitHub::New,
            github_config: None,
            issue: state::Issue::Selected(issue),
            packages: vec![package().0],
        };

        let command = replace_variables(command, variables, &state, Verbose::No).unwrap();

        assert_eq!(
            command,
            format!(
                "blah {} {}",
                &state.packages[0]
                    .get_version(Verbose::No)
                    .unwrap()
                    .into_latest()
                    .unwrap(),
                expected_branch_name
            )
        );
    }

    #[test]
    fn replace_version() {
        let command = "blah $$ other blah".to_string();
        let mut variables = IndexMap::new();
        variables.insert("$$".to_string(), Variable::Version);
        let state = State::new(None, None, vec![package().0]);

        let command = replace_variables(command, variables, &state, Verbose::No).unwrap();

        assert_eq!(
            command,
            format!(
                "blah {} other blah",
                &state.packages[0]
                    .get_version(Verbose::No)
                    .unwrap()
                    .into_latest()
                    .unwrap(),
            )
        );
    }

    #[test]
    fn replace_prepared_version() {
        let command = "blah $$ other blah".to_string();
        let mut variables = IndexMap::new();
        variables.insert("$$".to_string(), Variable::Version);
        let mut state = State::new(None, None, vec![package().0]);
        let version = Version::new(1, 2, 3, None);
        state.packages[0].prepared_release = Some(Release::new(None, version.clone()));

        let command = replace_variables(command, variables, &state, Verbose::No).unwrap();

        assert_eq!(command, format!("blah {version} other blah"));
    }

    #[test]
    fn replace_issue_branch() {
        let command = "blah $$ other blah".to_string();
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
            issue: state::Issue::Selected(issue),
            packages: Vec::new(),
        };

        let command = replace_variables(command, variables, &state, Verbose::No).unwrap();

        assert_eq!(command, format!("blah {expected_branch_name} other blah"));
    }

    #[test]
    fn replace_changelog_entry_prepared_release() {
        let command = "blah $$ other blah".to_string();
        let mut variables = IndexMap::new();
        variables.insert("$$".to_string(), Variable::ChangelogEntry);
        let mut state = State::new(None, None, vec![package().0]);
        let version = Version::new(1, 2, 3, None);
        let changelog_entry_body = "### Features\n#### Blah".to_string();
        state.packages[0].prepared_release =
            Some(Release::new(Some(changelog_entry_body), version.clone()));

        let command = replace_variables(command, variables, &state, Verbose::No).unwrap();

        let changelog_entry = state.packages[0]
            .prepared_release
            .as_ref()
            .unwrap()
            .new_changelog
            .clone()
            .unwrap();
        assert_eq!(command, format!("blah {changelog_entry} other blah"));
    }

    #[test]
    fn replace_changelog_entry_previous_release() {
        let command = "blah $$ other blah".to_string();
        let mut variables = IndexMap::new();
        variables.insert("$$".to_string(), Variable::ChangelogEntry);
        let (mut package, _temp_dir_guard) = package();
        let version = Version::new(1, 2, 3, None);
        let changelog_entry_body = "### Features\n#### Blah";
        let changelog_entry = format!("## {version} 2023-09-17\n\n{changelog_entry_body}");
        let changelog_path = package.changelog.take().unwrap().path;
        write(&changelog_path, &changelog_entry).unwrap();
        package.changelog = Some(changelog_path.try_into().unwrap()); // Have to reload content
        let state = State::new(None, None, vec![package]);

        let command = replace_variables(command, variables, &state, Verbose::No).unwrap();
        assert_eq!(command, format!("blah {changelog_entry_body} other blah"));
    }
}
