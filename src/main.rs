mod jira;
mod state;
mod workflow;

use crate::workflow::{Config, Step, Workflow};
use color_eyre::eyre::{eyre, Result, WrapErr};
use console::Term;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use dotenv::dotenv;
use state::State;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().unwrap();
    dotenv().ok();
    // TODO: Handle this error and print out a useful message about generating a token
    // TODO: store this in keychain instead of env var

    let config = workflow::load_workflow().await?;
    let Config {
        workflows,
        projects,
    } = config;
    let workflow = select_workflow(workflows)?;
    let state = State {
        projects,
        ..Default::default()
    };
    run_workflow(workflow, state).await
}

pub fn select_workflow(mut workflows: Vec<Workflow>) -> Result<Workflow> {
    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(
            &workflows
                .iter()
                .map(|flow| flow.name.as_str())
                .collect::<Vec<&str>>(),
        )
        .default(0)
        .with_prompt("Please select a workflow")
        .interact_on_opt(&Term::stdout())?;

    match selection {
        Some(index) => {
            let workflow = workflows.remove(index);
            Ok(workflow)
        }
        None => Err(eyre!("No workflow selected")),
    }
}

async fn run_workflow(workflow: Workflow, mut state: State) -> Result<()> {
    for step in workflow.steps.into_iter() {
        // TODO: Accumulate state and pass through steps
        state = run_step(step, state).await?;
    }
    Ok(())
}

async fn run_step(step: Step, state: State) -> Result<State> {
    match step {
        Step::ListIssues { status } => jira::select_issue(status, state).await,
    }
}
