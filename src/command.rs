use std::collections::HashMap;

use color_eyre::eyre::{eyre, Result, WrapErr};
use execute::shell;
use serde::Deserialize;

use crate::semver::get_version;
use crate::State;

/// Describes a value that you can replace an arbitrary string with when running a command.
#[derive(Debug, Deserialize)]
pub enum Variable {
    /// Uses the first supported version found in your project.
    ///
    /// ### Supported Formats
    /// 1. Cargo.toml `package.version`
    Version,
}

/// Run the command string `command` in the current shell after replacing the keys of `variables`
/// with the values that the [`Variable`]s represent.
pub(crate) fn run_command(
    state: State,
    mut command: String,
    variables: Option<HashMap<String, Variable>>,
) -> Result<State> {
    if let Some(variables) = variables {
        command =
            replace_variables(command, variables).wrap_err("While getting current version")?;
    }
    let status = shell(command).status()?;
    if status.success() {
        return Ok(state);
    }
    Err(eyre!("Got error status {} when running command", status))
}

/// Replace declared variables in the command string and return command.
fn replace_variables(mut command: String, variables: HashMap<String, Variable>) -> Result<String> {
    for (var_name, var_type) in variables.into_iter() {
        match var_type {
            Variable::Version => command = command.replace(&var_name, &get_version()?.to_string()),
        }
    }
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_variables() {
        let command = "blah $$".to_string();
        let mut variables = HashMap::new();
        variables.insert("$$".to_string(), Variable::Version);

        let command = replace_variables(command, variables).unwrap();

        assert_eq!(
            command,
            format!("blah {}", get_version().unwrap().to_string())
        )
    }
}
