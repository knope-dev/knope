use std::env::current_dir;
use std::io::Write;

use git_repository::object::Kind;
use git_repository::open;
use git_repository::refs::transaction::PreviousValue;

use crate::releases::Release;
use crate::step::StepError;
use crate::{RunType, State};

pub(crate) fn release(
    state: State,
    dry_run_stdout: Option<Box<dyn Write>>,
    release: &Release,
) -> Result<RunType, StepError> {
    let version_string = release.version.to_string();
    let tag = format!("v{}", version_string);

    if let Some(mut stdout) = dry_run_stdout {
        writeln!(stdout, "Would create Git tag {}", tag)?;
        return Ok(RunType::DryRun { stdout, state });
    }

    let repo = open(current_dir()?).map_err(|_e| StepError::NotAGitRepo)?;
    let head = repo.head_commit()?;
    repo.tag(tag, head.id, Kind::Commit, None, "", PreviousValue::Any)?;

    Ok(RunType::Real(state))
}
