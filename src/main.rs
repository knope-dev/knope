#![warn(clippy::all, clippy::pedantic, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]
#![forbid(unsafe_code)]

use color_eyre::eyre::{Result, WrapErr};

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

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().expect("Could not set up error handling with color_eyre");

    let Config {
        workflows,
        jira,
        github,
    } = Config::load("dobby.toml").wrap_err("Could not load config file at dobby.toml")?;
    let workflow = select(workflows, "Select a workflow")?;
    let state = State::new(jira, github);
    workflow::run_workflow(workflow, state).await
}
