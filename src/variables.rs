use indexmap::IndexMap;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};

use crate::{
    git::branch_name_from_issue,
    state,
    state::State,
    step::releases::{package, semver},
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
}

/// Replace declared variables in the string and return the new string.
pub(crate) fn replace_variables(
    mut command: String,
    variables: IndexMap<String, Variable>,
    state: &State,
    verbose: Verbose,
) -> Result<String, Error> {
    for (var_name, var_type) in variables {
        match var_type {
            Variable::Version => {
                let package = if state.packages.len() > 1 {
                    return Err(Error::TooManyPackages);
                } else if let Some(package) = state.packages.first() {
                    package
                } else {
                    return Err(package::Error::no_defined_packages_with_help().into());
                };
                let version = if let Some(release) = package.prepared_release.as_ref() {
                    release.new_version.to_string()
                } else {
                    package
                        .get_version(verbose)?
                        .into_latest()
                        .ok_or(Error::NoCurrentVersion)?
                        .to_string()
                };
                command = command.replace(&var_name, &version);
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

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("Too many packages defined")]
    #[diagnostic(
        code(command::too_many_packages),
        help("The Version variable in a Command step can only be used with a single [package].")
    )]
    TooManyPackages,
    #[error(transparent)]
    #[diagnostic(transparent)]
    Package(#[from] package::Error),
    #[error("Could not determine the current version of the package")]
    #[diagnostic(
        code(command::no_current_version),
        url("https://knope-dev.github.io/knope/config/packages.html#versioned_files")
    )]
    NoCurrentVersion,
    #[error("No issue selected")]
    #[diagnostic(
        code(git::no_issue_selected),
        help("The IssueBranch command variable requires selecting an issue first with SelectGitHubIssue or SelectJiraIssue")
    )]
    NoIssueSelected,
    #[error(transparent)]
    #[diagnostic(transparent)]
    SemVer(#[from] semver::Error),
}

#[cfg(test)]
mod test_replace_variables {
    use std::path::PathBuf;

    use super::*;
    use crate::{
        state,
        step::{
            issues::Issue,
            releases::{semver::Version, Package, Release},
        },
        workflow::Verbose,
    };

    fn packages() -> Vec<Package> {
        vec![Package {
            versioned_files: vec![PathBuf::from("Cargo.toml").try_into().unwrap()],
            changelog: Some(PathBuf::from("CHANGELOG.md").try_into().unwrap()),
            ..Package::default()
        }]
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
            packages: packages(),
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
        let state = State::new(None, None, packages());

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
        let mut state = State::new(None, None, packages());
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
}
