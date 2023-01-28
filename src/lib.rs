#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::cargo)]
#![allow(clippy::multiple_crate_versions)] // Let cargo-deny handle this
#![forbid(unsafe_code)]

use std::io::stdout;

use clap::Parser;
use miette::{miette, Result};
use prompt::select;

use crate::{
    config::Config,
    state::{RunType, State},
};

mod app_config;
mod command;
mod config;
mod git;
mod issues;
mod prompt;
mod releases;
mod state;
mod step;
mod workflow;

/// The main entry point for the application.
///
/// # Errors
///
/// 1. `knope.toml` not found
/// 2. `knope.toml` not valid
/// 3. Selected workflow not found
/// 4. Passthrough errors of selected workflow
pub fn run(cli: Cli) -> Result<()> {
    if cli.generate {
        println!("Generating a knope.toml file");
        let config = config::generate();
        return config.write_out();
    }

    let preselected_workflow = cli.workflow;

    let mut config = Config::load()?;

    if cli.upgrade {
        let upgraded = config.upgrade();
        return if upgraded {
            config.write_out()
        } else {
            println!("Nothing to upgrade");
            Ok(())
        };
    }

    if let Some(prerelease_label) = cli.prerelease_label {
        config.set_prerelease_label(&prerelease_label);
    }
    let packages = config.packages()?;
    let state = State::new(config.jira, config.github, packages);

    if cli.validate {
        workflow::validate(config.workflows, state)?;
        return Ok(());
    }

    let workflow_name = if let Some(workflow_name) = preselected_workflow {
        workflow_name
    } else {
        let workflow_names: Vec<&str> = config
            .workflows
            .iter()
            .map(|workflow| workflow.name.as_str())
            .collect();
        select(workflow_names, "Select a workflow").map(String::from)?
    };
    let workflow = config
        .workflows
        .into_iter()
        .find(|w| w.name == workflow_name)
        .ok_or_else(|| miette!("No workflow named {}", workflow_name))?;

    let state = if cli.dry_run {
        RunType::DryRun {
            state,
            stdout: Box::new(stdout()),
        }
    } else {
        RunType::Real(state)
    };

    workflow::run(workflow, state)?;
    Ok(())
}

/// The CLI application defined as a struct.
///
/// Use [`Cli::parse()`] to parse the command line arguments.
#[derive(Clone, Parser)]
#[command(long_about = None)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cli {
    /// Name a workflow to bypass the interactive select and just run it. If not provided,
    /// you'll be asked to select one.
    workflow: Option<String>,

    #[arg(long)]
    /// Pretend to run a workflow, outputting what _would_ happen without actually doing it.
    dry_run: bool,

    #[arg(long)]
    /// Generate a new `knope.toml` file.
    generate: bool,

    #[arg(long, env = "KNOPE_PRERELEASE_LABEL")]
    /// Set the `prerelease_label` attribute of any `PrepareRelease` steps at runtime.
    prerelease_label: Option<String>,

    #[arg(long)]
    /// Upgrade to the latest `knope.toml` syntax from any deprecated (but still supported) syntax.
    upgrade: bool,

    #[arg(long)]
    /// Check that the `knope.toml` file is valid.
    validate: bool,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn verify_app() {
        Cli::command().debug_assert();
    }
}
