use color_eyre::eyre::{eyre, ContextCompat, Result, WrapErr};

use crate::prompt::select;
use crate::state::State;
use git2::{Branch, BranchType, Repository};

pub fn create_branch(state: State) -> Result<State> {
    let data = match state {
        State::Initial(..) => return Err(eyre!("You must SelectIssue first.")),
        State::IssueSelected(data) => data,
    };
    let repo = match Repository::open(".") {
        Ok(repo) => repo,
        Err(e) => return Err(eyre!("Failed to find Git repo in this directory: {}", e)),
    };
    let new_branch_name = format!(
        "{}-{}",
        data.issue.key,
        data.issue.summary.to_ascii_lowercase()
    )
    .replace(" ", "-");
    let branches: Vec<(Branch, BranchType)> = repo
        .branches(None)
        .wrap_err("Could not list branches")?
        .into_iter()
        .filter_map(|b| b.ok())
        .collect();

    let existing = branches
        .iter()
        .map(|(b, _)| b)
        .find(|b| b.name().ok() == Some(Some(&new_branch_name)));

    if let Some(existing) = existing {
        let existing_ref = existing
            .get()
            .name()
            .ok_or_else(|| eyre!("Problem checking out existing branch"))?;
        repo.set_head(existing_ref).wrap_err_with(|| {
            format!("Found branch {} but could not set head.", new_branch_name)
        })?;
        return Ok(State::IssueSelected(data));
    }

    let branch_names: Vec<&str> = branches
        .iter()
        .map(|(b, _)| b.name())
        .filter_map(|name| name.ok())
        .filter_map(|name| name)
        .collect();

    let base_branch_name = select(branch_names, "Which branch do you want to base off of?")
        .wrap_err("failed to select branch")?;

    let (branch, _) = branches
        .iter()
        .find(|(b, _)| b.name().ok() == Some(Some(base_branch_name)))
        .wrap_err("failed to select branch")?;

    let new_branch = repo
        .branch(&new_branch_name, &branch.get().peel_to_commit()?, false)
        .wrap_err_with(|| format!("Failed to create new branch {}", new_branch_name))?;
    let ref_name = new_branch
        .get()
        .name()
        .ok_or_else(|| eyre!("Problem checking out existing branch"))?;
    repo.set_head(ref_name)
        .wrap_err_with(|| format!("Could not check out {} after creation", &ref_name))?;
    Ok(State::IssueSelected(data))
}
