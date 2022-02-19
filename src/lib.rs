#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::cargo)]
#![forbid(unsafe_code)]

use clap::{crate_authors, crate_description, crate_version, Arg, ArgMatches, Command};
use color_eyre::eyre::{ContextCompat, Result, WrapErr};

use prompt::select;

use crate::config::Config;
use crate::state::State;

mod app_config;
mod cargo;
mod command;
mod config;
mod git;
mod issues;
mod package_json;
mod prompt;
mod pyproject;
mod releases;
mod semver;
mod state;
mod step;
mod workflow;

/// The main entry point for the application.
///
/// # Errors
///
/// 1. `dobby.toml` not found
/// 2. `dobby.toml` not valid
/// 3. Selected workflow not found
/// 4. Passthrough errors of selected workflow
pub fn run(matches: &ArgMatches) -> Result<()> {
    let preselected_workflow = matches.value_of("WORKFLOW");

    let Config {
        workflows,
        jira,
        github,
    } = Config::load("dobby.toml").wrap_err("Could not load config file at dobby.toml")?;

    let workflow = match preselected_workflow {
        None => select(workflows, "Select a workflow")?,
        Some(name) => workflows
            .into_iter()
            .find(|w| w.name == name)
            .wrap_err_with(|| format!("No workflow named {}", name))?,
    };

    let state = State::new(jira, github);
    workflow::run_workflow(workflow, state)
}

/// Construct the Clap app for parsing args.
#[must_use]
pub fn command() -> clap::Command<'static> {
    Command::new("Dobby")
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::new("WORKFLOW")
                .help(
                    "Name a workflow to bypass the interactive select and just run it. \
                        If not provided, you'll be asked to select one",
                )
                .index(1),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_app() {
        command().debug_assert();
    }
}
