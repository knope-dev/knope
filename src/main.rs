//! Dobby is a CLI for developers used to automate common tasks workflows. Things like transitioning
//! issues, creating and merging branches, creating pull requests, bumping versions, tagging...
//! anything that is a repetitive, time consuming task in your development cycle, this tool is
//! made to speed up.
//!
//! ## How it Works
//! Basically you create a file called `dobby.toml` in your project directory which defines some
//! workflows. The format of this file has to match [`Config`], the key piece to which is
//! the `workflows` array. For a full example of a `dobby.toml`, check out the file for this project!
//!
//! Once you've got a config set up, you just run this program (`dobby` if you installed normally via
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
//! You define a [`Config`] which has some metadata (e.g. Jira details) about your project, as well
//! as a set of defined [`Workflow`]s. Each [`Workflow`] consists of a series of [`Step`]s that will
//! execute in order, stopping if any step fails. Steps can affect the [`State`] of the workflow. Some
//! [`Step`]s require that the workflow be in a specific [`State`] before they will work.

#![warn(clippy::all, clippy::pedantic, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use color_eyre::eyre::{Result, WrapErr};
use dotenv::dotenv;

pub use crate::semver::Rule;
pub use command::Variable;
pub use config::{Config, Jira};
use prompt::select;
pub use state::State;
pub use step::Step;
pub use workflow::Workflow;

mod cargo;
mod command;
mod config;
mod git;
mod jira;
mod prompt;
mod semver;
mod state;
mod step;
mod workflow;

fn main() -> Result<()> {
    color_eyre::install().expect("Could not set up error handling with color_eyre");
    dotenv().ok();

    let Config { workflows, jira } =
        Config::load("dobby.toml").wrap_err("Could not load config file at dobby.toml")?;
    let workflow = select(workflows, "Select a workflow")?;
    let state = State::new(jira);
    workflow::run_workflow(workflow, state)
}
