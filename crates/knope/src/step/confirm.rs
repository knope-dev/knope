use inquire::Confirm;
use miette::Diagnostic;

use crate::{app_config, RunType};

pub(crate) fn confirm(
    mut run_type: RunType,
    message: &str,
    assume_yes: bool,
) -> Result<RunType, Error> {
    if assume_yes {
        return Ok(run_type);
    }
    let (_, dry_run_stdout) = match &mut run_type {
        RunType::DryRun { state, stdout } => (state, Some(stdout)),
        RunType::Real(state) => (state, None),
    };

    if let Some(stdout) = dry_run_stdout {
        writeln!(stdout, "Would prompt for the following message {message}")?;
        return Ok(run_type);
    }

    let confirmation = Confirm::new(message).with_default(true).prompt();

    match confirmation {
        Ok(true) => Ok(run_type),
        Ok(false) => Err(Error::Confirm),
        Err(err) => Err(Error::Prompt(err)),
    }
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("User did not confirm")]
    Confirm,
    #[error(transparent)]
    Prompt(#[from] inquire::InquireError),
    #[error("Unable to write to stdout: {0}")]
    Stdout(#[from] std::io::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    AppConfig(#[from] app_config::Error),
}
