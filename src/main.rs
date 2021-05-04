#![warn(clippy::all, clippy::pedantic, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]
#![forbid(unsafe_code)]

use clap::{crate_authors, crate_description, crate_version, App, Arg};
use color_eyre::eyre::{ContextCompat, Result, WrapErr};

use crate::config::Config;
use crate::state::State;
use prompt::select;

mod app_config;
mod cargo;
mod changelog;
mod command;
mod config;
mod conventional_commits;
mod git;
mod issues;
mod package_json;
mod prompt;
mod pyproject;
mod semver;
mod state;
mod step;
mod workflow;

fn main() -> Result<()> {
    color_eyre::install().expect("Could not set up error handling with color_eyre");

    let matches = App::new("Dobby")
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::with_name("WORKFLOW")
                .help("Name a workflow to bypass the interactive select and just run it.")
                .index(1),
        )
        .get_matches();

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
