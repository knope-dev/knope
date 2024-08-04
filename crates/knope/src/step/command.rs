use indexmap::IndexMap;
use miette::Diagnostic;
use tracing::info;

use crate::{
    state::State,
    variables,
    variables::{replace_variables, Template, Variable},
    RunType,
};

/// Run the command string `command` in the current shell after replacing the keys of `variables`
/// with the values that the [`Variable`]s represent.
pub(crate) fn run_command(
    state: RunType<State>,
    mut command: String,
    shell: bool,
    variables: Option<IndexMap<String, Variable>>,
) -> Result<RunType<State>, Error> {
    let (run_type, mut state) = state.take();
    if let Some(variables) = variables {
        command = replace_variables(
            Template {
                template: command,
                variables,
            },
            &mut state,
        )?;
    }
    if let RunType::DryRun(()) = run_type {
        info!("Would run {command}");
        return Ok(run_type.of(state));
    }
    let status = if shell {
        execute::shell(command).status()?
    } else {
        execute::command(command).status()?
    };
    if status.success() {
        return Ok(run_type.of(state));
    }
    Err(Error::Command(status))
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("Command returned non-zero exit code")]
    #[diagnostic(
        code(command::failed),
        help("The command failed to execute. Try running it manually to get more information.")
    )]
    Command(std::process::ExitStatus),
    #[error("I/O error: {0}")]
    #[diagnostic(code(command::io))]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Variables(#[from] variables::Error),
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test_run_command {

    use super::*;
    use crate::State;

    #[test]
    fn test() {
        let command = "echo \"hello\"";
        let result = run_command(
            RunType::Real(State::new(None, None, None, Vec::new(), Vec::new())),
            command.to_string(),
            false,
            None,
        );

        assert!(result.is_ok());

        let result = run_command(
            RunType::Real(State::new(None, None, None, Vec::new(), Vec::new())),
            String::from("exit 1"),
            false,
            None,
        );
        assert!(result.is_err());
    }
}
