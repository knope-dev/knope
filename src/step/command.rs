use indexmap::IndexMap;
use miette::Diagnostic;

use crate::{
    variables,
    variables::{replace_variables, Template, Variable},
    RunType,
};

/// Run the command string `command` in the current shell after replacing the keys of `variables`
/// with the values that the [`Variable`]s represent.
pub(crate) fn run_command(
    mut run_type: RunType,
    mut command: String,
    variables: Option<IndexMap<String, Variable>>,
) -> Result<RunType, Error> {
    let (state, dry_run_stdout) = match &mut run_type {
        RunType::DryRun { state, stdout } => (state, Some(stdout)),
        RunType::Real(state) => (state, None),
    };
    if let Some(variables) = variables {
        command = replace_variables(
            Template {
                template: command,
                variables,
            },
            state,
        )?;
    }
    if let Some(stdout) = dry_run_stdout {
        writeln!(stdout, "Would run {command}")?;
        return Ok(run_type);
    }
    let status = execute::command(command).status()?;
    if status.success() {
        return Ok(run_type);
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
mod test_run_command {
    use tempfile::NamedTempFile;

    use super::*;
    use crate::{workflow::Verbose, State};

    #[test]
    fn test() {
        let file = NamedTempFile::new().unwrap();
        let command = format!("cat {}", file.path().to_str().unwrap());
        let result = run_command(
            RunType::Real(State::new(None, None, None, Vec::new(), Verbose::No)),
            command.clone(),
            None,
        );

        assert!(result.is_ok());

        file.close().unwrap();

        let result = run_command(
            RunType::Real(State::new(None, None, None, Vec::new(), Verbose::No)),
            command,
            None,
        );
        assert!(result.is_err());
    }
}
