use std::{collections::HashSet, io::Write, path::PathBuf};

use changesets::{ChangeSet, UniqueId, Versioning};
use inquire::{MultiSelect, Select};
use itertools::Itertools;
use knope_versioning::changes::{ChangeType, CHANGESET_DIR, DEFAULT_PACKAGE_NAME};
use miette::Diagnostic;

use super::Package;
use crate::{dry_run::DryRun, fs, prompt, state::RunType};

pub(crate) fn create_change_file(run_type: RunType) -> Result<RunType, Error> {
    let state = match run_type {
        RunType::DryRun { state, mut stdout } => {
            write!(&mut stdout, "Would create a new change file").map_err(fs::Error::Stdout)?;
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
            let package_name = package.name;
            let change_types = package
                .versioning
                .changelog_sections
                .iter()
                .flat_map(|(_, sources)| sources.iter().filter_map(ChangeType::to_changeset_type))
                .collect_vec();
            let prompt = if let Some(package_name) = package_name.as_ref() {
                format!("What type of change is this for {package_name}?")
            } else {
                "What type of change is this?".to_string()
            };
            Select::new(&prompt, change_types)
                .prompt()
                .map_err(prompt::Error::from)
                .map_err(Error::from)
                .map(|change_type| (package_name.unwrap_or_default().to_string(), change_type))
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
        fs::create_dir(&mut None, &changeset_path)?;
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

// TODO: Move some of this to knope_versioning
pub(crate) fn add_releases_from_changeset(
    packages: Vec<Package>,
    is_prerelease: bool,
    dry_run: DryRun,
) -> Result<Vec<Package>, Error> {
    let changeset_path = PathBuf::from(CHANGESET_DIR);
    if !changeset_path.exists() {
        return Ok(packages);
    }
    let releases: Vec<changesets::Release> = ChangeSet::from_directory(&changeset_path)?.into();
    let mut changesets_deleted = HashSet::new();
    Ok(packages
        .into_iter()
        .map(|mut package| {
            if let Some(release_changes) = releases.iter().find(|release| {
                release.package_name == package.name.as_deref().unwrap_or(DEFAULT_PACKAGE_NAME)
            }) {
                package
                    .pending_changes
                    .extend(release_changes.changes.clone().into_iter().map(|change| {
                        let file_name = change.unique_id.to_file_name();
                        if !changesets_deleted.contains(&file_name) && !is_prerelease {
                            if let Some(dry_run) = dry_run {
                                writeln!(
                                    dry_run,
                                    "Would delete: {}",
                                    changeset_path.join(&file_name).display()
                                )
                                .ok(); // Truly not the end of the world if stdio fails, and error handling is hard
                            } else {
                                std::fs::remove_file(changeset_path.join(&file_name)).ok();
                            }
                            changesets_deleted.insert(file_name);
                        }
                        change.into()
                    }));
            }
            package
        })
        .collect())
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(
        code(changesets::could_not_read_changeset),
        help(
            "This could be a file-system issue or a problem with the formatting of a change file."
        )
    )]
    CouldNotReadChangeSet(#[from] changesets::LoadingError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Prompt(#[from] prompt::Error),
}
