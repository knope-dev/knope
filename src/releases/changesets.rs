use std::{fmt, io::Write, path::PathBuf};

use changesets::{ChangeSet, UniqueId, Versioning};
use inquire::{MultiSelect, Select};
use itertools::Itertools;

use crate::{
    prompt,
    releases::{package::ChangelogSectionSource, Change, Package},
    state::RunType,
    step::StepError,
};

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
        .map_err(prompt::Error::from)?
    };

    let versioning = packages
        .into_iter()
        .map(|package| {
            let package_name = package.to_string();
            let change_types = [ChangeType::Breaking, ChangeType::Feature, ChangeType::Fix]
                .into_iter()
                .chain(
                    package
                        .extra_changelog_sections
                        .into_keys()
                        .filter_map(|key| {
                            if let ChangelogSectionSource::CustomChangeType(_) = &key {
                                Some(ChangeType::Custom(key))
                            } else {
                                None
                            }
                        }),
                )
                .collect_vec();
            Select::new("What type of change is this?", change_types)
                .prompt()
                .map_err(prompt::Error::from)
                .map_err(StepError::from)
                .map(|change_type| (package_name, change_type.into()))
        })
        .collect::<Result<Versioning, StepError>>()?;
    let summary = inquire::Text::new("What is a short summary of this change?")
        .with_help_message("This will be used as a header in the changelog")
        .prompt()
        .map_err(prompt::Error::from)?;
    let unique_id = UniqueId::from(&summary);
    let summary = format!("#### {summary}");
    let change = changesets::Change {
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
        let file_name = change.unique_id.to_file_name();
        StepError::CouldNotCreateFile(changeset_path.join(file_name))
    })?;
    Ok(RunType::Real(state))
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) enum ChangeType {
    Breaking,
    Feature,
    Fix,
    Custom(ChangelogSectionSource),
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
            ChangeType::Custom(custom) => Self::Custom(custom.to_string()),
        }
    }
}

impl From<&changesets::ChangeType> for ChangeType {
    fn from(value: &changesets::ChangeType) -> Self {
        match value {
            changesets::ChangeType::Major => Self::Breaking,
            changesets::ChangeType::Minor => Self::Feature,
            changesets::ChangeType::Patch => Self::Fix,
            changesets::ChangeType::Custom(custom) => Self::Custom(
                ChangelogSectionSource::CustomChangeType(custom.clone().into()),
            ),
        }
    }
}

impl From<ChangelogSectionSource> for ChangeType {
    fn from(source: ChangelogSectionSource) -> Self {
        Self::Custom(source)
    }
}

pub(crate) const DEFAULT_CHANGESET_PACKAGE_NAME: &str = "default";

pub(crate) fn add_releases_from_changeset(
    packages: Vec<Package>,
    dry_run: &mut Option<Box<dyn Write>>,
) -> Result<Vec<Package>, StepError> {
    let changeset_path = PathBuf::from(".changeset");
    if !changeset_path.exists() {
        return Ok(packages);
    }
    let mut changeset = ChangeSet::from_directory(&changeset_path)?;
    Ok(packages
        .into_iter()
        .map(|mut package| {
            if let Some(release_changes) = changeset.releases.remove(
                package
                    .name
                    .as_deref()
                    .unwrap_or(DEFAULT_CHANGESET_PACKAGE_NAME),
            ) {
                package
                    .pending_changes
                    .extend(release_changes.changes.into_iter().map(|change| {
                        if let Some(dry_run) = dry_run {
                            writeln!(
                                dry_run,
                                "Would delete: {}",
                                changeset_path
                                    .join(change.unique_id.to_file_name())
                                    .display()
                            )
                            .ok(); // Truly not the end of the world if stdio fails, and error handling is hard
                        } else {
                            // Error is ignored because we will attempt to double-delete some files.
                            std::fs::remove_file(
                                changeset_path.join(change.unique_id.to_file_name()),
                            )
                            .ok();
                        }
                        Change::ChangeSet(change)
                    }));
            }
            package
        })
        .collect())
}
