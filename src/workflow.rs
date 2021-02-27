use color_eyre::Result;
use serde::Deserialize;

use crate::step::{run_step, Step};
use crate::State;

/// Run a [`Workflow`], updating the passed in `state` for each step.
pub(crate) fn run_workflow(workflow: Workflow, mut state: State) -> Result<()> {
    for step in workflow.steps {
        state = run_step(step, state)?;
    }
    Ok(())
}

/// A workflow is basically the state machine to run for a single execution of Dobby.
#[derive(Deserialize, Debug)]
pub(crate) struct Workflow {
    /// The display name of this Workflow. This is what you'll see when you go to select it.
    pub(crate) name: String,
    /// A list of [`Step`]s to execute in order, stopping if any step fails.
    pub(crate) steps: Vec<Step>,
}

impl std::fmt::Display for Workflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.name)
    }
}
