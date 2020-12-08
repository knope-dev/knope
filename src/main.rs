//! Flow is a CLI for developers used to automate common tasks workflows. Things like transitioning
//! issues, creating and merging branches, creating pull requests, bumping versions, tagging...
//! anything that is a repetitive, time consuming task in your development cycle, this tool is
//! made to speed up.
//!
//! ## How it Works
//! Basically you create a file called `flow.toml` in your project directory which defines some
//! workflows. The format of this file has to match [`Config`], the key piece to which is
//! the `workflows` array. For a full example of a `flow.toml`, check out the file for this project!
//!
//! Once you've got a config set up, you just run this program (`flow` if you installed normally via
//! cargo). That will prompt you to select one of your configured workflows. Do that and you're
//! off to the races!
//!
//! ## Features
//! More detail on everything this program can do can be found by digging into [`Config`] but
//! here's a rough (incomplete) summary:
//!
//! 1. Select issues from Jira to work on, transition and create branches from them.
//! 2. Do some basic git commands like switching branches or rebasing.
//! 3. Bump the version of your project using semantic rules.
//! 4. Do whatever you want by running arbitrary shell commands and substituting data from the project!
//!
//! ## Concepts
//! You define a [`Config'] which has some metadata (e.g. Jira details) about your project, as well
//! as a set of defined [`Workflow`]s. Each [`Workflow`] consists of a series of [`Step`]s that will
//! execute in order, stopping if any step fails. Steps can affect the [`State`] of the workflow. Some
//! [`Step`]s require that the workflow be in a specific [`State`] before they will work.

use color_eyre::eyre::{Result, WrapErr};
use dotenv::dotenv;

pub use config::Config;
use prompt::select;
use state::State;

mod cargo;
pub mod command;
pub mod config;
mod git;
mod jira;
mod prompt;
mod semver;
pub mod state;
pub mod step;
pub mod workflow;

fn main() -> Result<()> {
    color_eyre::install().unwrap();
    dotenv().ok();

    let Config { workflows, jira } =
        Config::load("flow.toml").wrap_err("Could not load config file at flow.toml")?;
    let workflow = select(workflows, "Select a workflow")?;
    let state = State::new(jira);
    workflow::run_workflow(workflow, state)
}
