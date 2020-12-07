use color_eyre::eyre::{Result, WrapErr};

pub use crate::command::Variable;
pub use crate::prompt::select;
pub use crate::state::State;
pub use crate::workflow::{Config, Step, Workflow};

mod cargo;
mod command;
mod git;
mod jira;
mod prompt;
mod semver;
mod state;
mod workflow;

pub fn run_workflow(workflow: Workflow, mut state: State) -> Result<()> {
    for step in workflow.steps.into_iter() {
        state = run_step(step, state)?;
    }
    Ok(())
}

fn run_step(step: Step, state: State) -> Result<State> {
    match step {
        Step::SelectIssue { status } => {
            jira::select_issue(status, state).wrap_err("During SelectIssue")
        }
        Step::TransitionIssue { status } => {
            jira::transition_selected_issue(status, state).wrap_err("During TransitionIssue")
        }
        Step::SwitchBranches => git::switch_branches(state).wrap_err("During SwitchBranches"),
        Step::RebaseBranch { to } => git::rebase_branch(state, to).wrap_err("During MergeBranch"),
        Step::BumpVersion(rule) => {
            crate::semver::bump_version(state, rule).wrap_err("During BumpVersion")
        }
        Step::Command { command, variables } => {
            command::run_command(state, command, variables).wrap_err("During Command")
        }
    }
}
