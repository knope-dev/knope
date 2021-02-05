use std::collections::HashMap;

use color_eyre::eyre::{eyre, Result, WrapErr};
use execute::shell;
use serde::Deserialize;

use crate::git::branch_name_from_issue;
use crate::semver::get_version;
use crate::State;

/// Describes a value that you can replace an arbitrary string with when running a command.
#[derive(Debug, Deserialize)]
pub(crate) enum Variable {
    /// Uses the first supported version found in your project.
    Version,
    /// The generated branch name for the selected issue. Note that this means the workflow must
    /// already be in [`State::IssueSelected`] when this variable is used.
    IssueBranch,
}

/// Run the command string `command` in the current shell after replacing the keys of `variables`
/// with the values that the [`Variable`]s represent.
pub(crate) fn run_command(
    state: State,
    mut command: String,
    variables: Option<HashMap<String, Variable>>,
) -> Result<State> {
    if let Some(variables) = variables {
        command = replace_variables(command, variables, &state)
            .wrap_err("While getting current version")?;
    }
    let status = shell(command).status()?;
    if status.success() {
        return Ok(state);
    }
    Err(eyre!("Got error status {} when running command", status))
}

/// Replace declared variables in the command string and return command.
fn replace_variables(
    mut command: String,
    variables: HashMap<String, Variable>,
    state: &State,
) -> Result<String> {
    for (var_name, var_type) in variables {
        match var_type {
            Variable::Version => command = command.replace(&var_name, &get_version()?.to_string()),
            Variable::IssueBranch => {
                match state {
                    State::Initial(_) => return Err(eyre!("Cannot use the variable IssueBranch unless the current workflow state is IssueSelected")),
                    State::IssueSelected(state_data) => {
                        command = command.replace(&var_name, &branch_name_from_issue(&state_data.issue))
                    }
                }
            }
        }
    }
    Ok(command)
}

#[cfg(test)]
mod test_run_command {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test() {
        let file = NamedTempFile::new().unwrap();
        let command = format!("cat {}", file.path().to_str().unwrap());
        let result = run_command(State::new(None, None), command.clone(), None);

        assert!(result.is_ok());

        file.close().unwrap();

        let result = run_command(State::new(None, None), command, None);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod test_replace_variables {
    use super::*;
    use crate::issues::Issue;
    use crate::state::{GitHub, Initial, IssueSelected};

    #[test]
    fn multiple_variables() {
        let command = "blah $$ branch_name".to_string();
        let mut variables = HashMap::new();
        variables.insert("$$".to_string(), Variable::Version);
        variables.insert("branch_name".to_string(), Variable::IssueBranch);
        let issue = Issue {
            key: "13".to_string(),
            summary: "1234".to_string(),
        };
        let expected_branch_name = branch_name_from_issue(&issue);
        let state = State::IssueSelected(IssueSelected {
            jira_config: None,
            github_state: GitHub::New,
            github_config: None,
            issue,
        });

        let command = replace_variables(command, variables, &state).unwrap();

        assert_eq!(
            command,
            format!(
                "blah {} {}",
                get_version().unwrap().to_string(),
                expected_branch_name
            )
        )
    }

    #[test]
    fn replace_version() {
        let command = "blah $$ other blah".to_string();
        let mut variables = HashMap::new();
        variables.insert("$$".to_string(), Variable::Version);
        let state = State::Initial(Initial {
            jira_config: None,
            github_state: GitHub::New,
            github_config: None,
        });

        let command = replace_variables(command, variables, &state).unwrap();

        assert_eq!(
            command,
            format!("blah {} other blah", get_version().unwrap().to_string(),)
        )
    }

    #[test]
    fn replace_issue_branch() {
        let command = "blah $$ other blah".to_string();
        let mut variables = HashMap::new();
        variables.insert("$$".to_string(), Variable::IssueBranch);
        let issue = Issue {
            key: "13".to_string(),
            summary: "1234".to_string(),
        };
        let expected_branch_name = branch_name_from_issue(&issue);
        let state = State::IssueSelected(IssueSelected {
            jira_config: None,
            github_state: GitHub::New,
            github_config: None,
            issue,
        });

        let command = replace_variables(command, variables, &state).unwrap();

        assert_eq!(command, format!("blah {} other blah", expected_branch_name))
    }
}
