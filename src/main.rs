use color_eyre::eyre::{Result, WrapErr};
use dotenv::dotenv;

use crate::prompt::select;
use crate::state::{Initial, State};
use crate::workflow::{Config, Step, Workflow};

mod git;
mod jira;
mod prompt;
mod state;
mod workflow;

fn main() -> Result<()> {
    color_eyre::install().unwrap();
    dotenv().ok();

    let config = workflow::load_workflow()?;
    let Config { workflows, jira } = config;
    let workflow = select(workflows, "Select a workflow")?;
    let state = State::Initial(Initial { jira_config: jira });
    run_workflow(workflow, state)
}

fn run_workflow(workflow: Workflow, mut state: State) -> Result<()> {
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
        Step::CreateBranch => git::create_branch(state).wrap_err("During CreateBranch"),
    }
}
