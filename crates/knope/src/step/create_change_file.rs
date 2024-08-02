use std::path::PathBuf;

use changesets::{UniqueId, Versioning};
use inquire::{MultiSelect, Select};
use itertools::Itertools;
use knope_versioning::changes::{ChangeType, CHANGESET_DIR};
use miette::Diagnostic;
use tracing::info;

use crate::{
    fs, prompt,
    state::{RunType, State},
};

pub(crate) fn run(state: RunType<State>) -> Result<RunType<State>, Error> {
    let state = match state {
        RunType::DryRun(state) => {
            info!("Would create a new change file");
            return Ok(RunType::DryRun(state));
        }
        RunType::Real(state) => state,
    };

    let packages = if state.packages.len() == 1 {
        state.packages.iter().collect()
    } else {
        MultiSelect::new(
            "Which packages does this change affect?",
            state.packages.iter().collect(),
        )
        .prompt()
        .map_err(prompt::Error::from)?
    };

    let versioning = packages
        .iter()
        .map(|package| {
            let package_name = package.name();
            let change_types = package
                .versioning
                .release_notes
                .sections
                .iter()
                .flat_map(|(_, sources)| sources.iter().filter_map(ChangeType::to_changeset_type))
                .collect_vec();
            let prompt = if let Some(package_name) = package_name.as_custom() {
                format!("What type of change is this for {package_name}?")
            } else {
                "What type of change is this?".to_string()
            };
            Select::new(&prompt, change_types)
                .prompt()
                .map_err(prompt::Error::from)
                .map_err(Error::from)
                .map(|change_type| (package_name.to_string(), change_type))
        })
        .collect::<Result<Versioning, Error>>()?;
    let summary = inquire::Text::new("What is a short summary of this change?")
        .with_help_message("This will be used as a header in the changelog")
        .prompt()
        .map_err(prompt::Error::from)?;
    let unique_id = UniqueId::from(&summary);
    let summary = format!("# {summary}");
    let change = changesets::Change {
        unique_id,
        versioning,
        summary,
    };

    let changeset_path = PathBuf::from(CHANGESET_DIR);
    if !changeset_path.exists() {
        fs::create_dir(RunType::Real(&changeset_path))?;
    }
    change
        .write_to_directory(&changeset_path)
        .map_err(|source| {
            let file_name = change.unique_id.to_file_name();
            fs::Error::Write {
                path: changeset_path.join(file_name),
                source,
            }
        })?;
    Ok(RunType::Real(state))
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Prompt(#[from] prompt::Error),
}
