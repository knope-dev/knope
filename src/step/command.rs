use std::path::PathBuf;

use indexmap::IndexMap;
use miette::Diagnostic;

use crate::{
    config::Config,
    variables::{self, replace_variables, Template, Variable},
    RunType,
};

/// Gets the path to use for the command, defaulting to the current working directory if `use_working_directory` isn't set.
/// If `use_working_directory` is set to `true`, the current working directory is used.
/// If `use_working_directory` is set to `false`, the directory of the first config file in the ancestry of the current working directory is used.
/// If there is no config file in the ancestry of the current working directory, the current working directory is used. Although this situation should be impossible,
/// as the user will need to configure the command explicitly to set `use_working_directory` to `false`.
fn get_directory_for_command(use_working_directory: Option<bool>) -> Option<PathBuf> {
    let use_working_directory_thing = use_working_directory.unwrap_or(true);
    if use_working_directory_thing {
        return None;
    }
    let config_path = Config::config_path();
    let config_directory = match &config_path {
        Some(path) => path.parent(),
        None => None,
    };

    config_directory.as_ref().map(|path| path.to_path_buf())
}

/// Run the command string `command` in the current shell after replacing the keys of `variables`
/// with the values that the [`Variable`]s represent.
pub(crate) fn run_command(
    mut run_type: RunType,
    mut command: String,
    variables: Option<IndexMap<String, Variable>>,
    use_working_directory: Option<bool>,
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
    let mut cmd = execute::command(command);

    let directory = get_directory_for_command(use_working_directory);

    println!("Directory: {directory:?}");
    if let Some(directory) = directory {
        cmd.current_dir(directory);
    }

    let status = cmd.status()?;
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

    use super::*;
    use crate::{workflow::Verbose, State};

    #[test]
    fn test() {
        let command = "echo \"hello\"";
        let result = run_command(
            RunType::Real(State::new(None, None, None, Vec::new(), Verbose::No)),
            command.to_string(),
            None,
            None,
        );

        assert!(result.is_ok());

        let result = run_command(
            RunType::Real(State::new(None, None, None, Vec::new(), Verbose::No)),
            String::from("exit 1"),
            None,
            None,
        );
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod test_get_directory_for_command {
    use super::get_directory_for_command;

    #[test]
    fn test_get_directory_for_command_with_use_working_directory_true_uses_working_directory() {
        let result = get_directory_for_command(Some(true));
        assert!(result.is_none());
    }

    #[test]
    fn test_get_directory_for_command_with_use_working_directory_false_uses_config_directory() {
        let result = get_directory_for_command(Some(false));
        assert!(result.is_some());
    }

    #[test]
    fn test_get_directory_for_command_without_use_working_directory_uses_current_directory() {
        let result = get_directory_for_command(None);
        assert!(result.is_none());
    }

    #[test]
    fn test_get_directory_for_command_with_no_config_file_in_ancestry_uses_current_directory() {
        let result = get_directory_for_command(None);
        assert!(result.is_none());
    }
}
