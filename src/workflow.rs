use color_eyre::Result;
use serde::export::Formatter;
use serde::Deserialize;

use crate::step::{run_step, Step};
use crate::State;

pub(crate) async fn run_workflow(workflow: Workflow, mut state: State) -> Result<()> {
    for step in workflow.steps {
        state = run_step(step, state).await?;
    }
    Ok(())
}

/// A workflow is the entrypoint to doing work with Dobby. Once you start running `dobby` you must
/// immediately select a workflow (by name) to be executed. A workflow consists of a series of
/// [`Step`]s that will run in order, stopping only if one step fails.
///
/// ## Example
/// ```toml
/// # dobby.toml
///
/// [[workflows]]
/// name = "My First Workflow"
///     [[workflows.steps]]
///     # First step details here
///     [[workflows.steps]]
///     # second step details here
/// ```
///
/// ## See Also
/// - [`Step`] for details on how each Step is defined.
#[derive(Deserialize, Debug)]
pub struct Workflow {
    /// The display name of this Workflow. This is what you'll see when you go to select it.
    pub name: String,
    /// A list of [`Step`]s to execute in order, stopping if any step fails.
    pub steps: Vec<Step>,
}

impl std::fmt::Display for Workflow {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.name)
    }
}
