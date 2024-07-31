use std::fmt::Debug;

use itertools::Itertools;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{state::RunType, step, step::Step, State};

/// A workflow is basically the state machine to run for a single execution of knope.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Workflow {
    /// The display name of this Workflow. This is what you'll see when you go to select it.
    pub(crate) name: String,
    /// The help text for this workflow. When running `knope --help`, this will be displayed.
    pub(crate) help_text: Option<String>,
    /// A list of [`Step`]s to execute in order, stopping if any step fails.
    pub(crate) steps: Vec<Step>,
}

impl Workflow {
    /// Set `prerelease_label` for any steps that are `PrepareRelease` steps.
    pub(crate) fn set_prerelease_label(&mut self, prerelease_label: &str) {
        for step in &mut self.steps {
            step.set_prerelease_label(prerelease_label);
        }
    }
}

/// A collection of errors from running with the `--validate` option.
#[derive(Debug, Error, Diagnostic)]
#[error("There are problems with the defined workflows")]
pub struct ValidationErrorCollection {
    #[related]
    errors: Vec<Error>,
}

/// An error from running or validating a single workflow.
#[derive(Debug, thiserror::Error, Diagnostic)]
#[error("Problem with workflow {name}")]
pub struct Error {
    name: String,
    #[related]
    inner: Box<[step::Error; 1]>,
}

/// Run a series of [`Step`], each of which updates `state`.
pub(crate) fn run(workflow: Workflow, mut state: RunType<State>) -> Result<(), Error> {
    for step in workflow.steps {
        state = match step.run(state) {
            Ok(state) => state,
            Err(err) => {
                return Err(Error {
                    name: workflow.name,
                    inner: Box::new([err]),
                });
            }
        };
    }
    Ok(())
}

#[allow(clippy::needless_pass_by_value)] // Lifetime errors if State is passed by ref.
pub(crate) fn validate(
    workflows: Vec<Workflow>,
    state: State,
) -> Result<(), ValidationErrorCollection> {
    let errors = workflows
        .into_iter()
        .filter_map(|workflow| run(workflow, RunType::DryRun(state.clone())).err())
        .collect_vec();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationErrorCollection { errors })
    }
}

impl std::fmt::Display for Workflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.name)
    }
}
