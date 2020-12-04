use std::collections::HashMap;

use color_eyre::eyre::{eyre, Result};
use execute::shell;
use serde::Deserialize;

use crate::semver::get_version;
use crate::State;

#[derive(Debug, Deserialize)]
pub enum Variable {
    Version,
}

pub(crate) fn run_command(
    state: State,
    mut command: String,
    variables: Option<HashMap<String, Variable>>,
) -> Result<State> {
    if let Some(variables) = variables {
        for (var_name, var_type) in variables.into_iter() {
            match var_type {
                Variable::Version => {
                    command = command.replace(&var_name, &get_version()?.to_string())
                }
            }
        }
    }
    let status = shell(command).status()?;
    if status.success() {
        return Ok(state);
    }
    Err(eyre!("Got error status {} when running command", status))
}
