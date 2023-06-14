use std::{fmt, path::PathBuf};

use changesets::{Change, Versioning};
use inquire::{MultiSelect, Select};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{state::RunType, step::StepError};

pub(crate) fn create_change_file(run_type: RunType) -> Result<RunType, StepError> {
    let state = match run_type {
        RunType::DryRun { state, mut stdout } => {
            write!(&mut stdout, "Would create a new change file")?;
            return Ok(RunType::DryRun { state, stdout });
        }
        RunType::Real(state) => state,
    };

    let packages = if state.packages.len() == 1 {
        state.packages.clone()
    } else {
        MultiSelect::new(
            "Which packages does this change affect?",
            state.packages.clone(),
        )
        .prompt()
        .map_err(StepError::UserInput)?
    };

    let versioning = packages
        .into_iter()
        .map(|package| {
            let package_name = package.to_string();
            let change_types = [ChangeType::Breaking, ChangeType::Feature, ChangeType::Fix]
                .into_iter()
                .chain(package.change_types.into_keys())
                .collect_vec();
            Select::new("What type of change is this?", change_types)
                .prompt()
                .map_err(StepError::UserInput)
                .map(|change_type| (package_name, change_type.into()))
        })
        .collect::<Result<Versioning, StepError>>()?;
    let summary = inquire::Text::new("What is a short summary of this change?")
        .with_help_message("This will be used as a header in the changelog")
        .prompt()
        .map_err(StepError::UserInput)?;
    let unique_id: String = create_unique_id(&summary);
    let summary = format!("#### {summary}");
    let change = Change {
        unique_id,
        versioning,
        summary,
    };

    let changeset_path = PathBuf::from(".changeset");
    if !changeset_path.exists() {
        std::fs::create_dir(&changeset_path)
            .map_err(|_| StepError::CouldNotCreateFile(changeset_path.clone()))?;
    }
    change.write_to_directory(&changeset_path).map_err(|_| {
        let file_name = change.file_name();
        StepError::CouldNotCreateFile(changeset_path.join(file_name))
    })?;
    Ok(RunType::Real(state))
}

fn create_unique_id(summary: &str) -> String {
    summary
        .chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c == ' ' {
                Some('_')
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
#[test]
fn test_create_unique_id() {
    assert_eq!(
        create_unique_id("`[i carry your heart with me(i carry it in]`"),
        "i_carry_your_heart_with_mei_carry_it_in"
    );
}

#[derive(Clone, Debug, Deserialize, Hash, Eq, PartialEq, Serialize)]
pub(crate) enum ChangeType {
    Breaking,
    Feature,
    Fix,
    Custom(String),
}

impl fmt::Display for ChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Breaking => write!(f, "breaking"),
            Self::Feature => write!(f, "feature"),
            Self::Fix => write!(f, "fix"),
            Self::Custom(custom) => write!(f, "{custom}"),
        }
    }
}

impl From<ChangeType> for changesets::ChangeType {
    fn from(value: ChangeType) -> Self {
        match value {
            ChangeType::Breaking => Self::Major,
            ChangeType::Feature => Self::Minor,
            ChangeType::Fix => Self::Patch,
            ChangeType::Custom(custom) => Self::Custom(custom),
        }
    }
}
