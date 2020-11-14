use color_eyre::eyre::{eyre, ContextCompat, Result, WrapErr};

use crate::prompt::select;
use crate::state::{Issue, State};
use git2::{Branch, Repository};

/// Based on the selected issue, either checks out an existing branch matching the name or creates
/// a new one, prompting for which branch to base it on.
pub fn switch_branches(state: State) -> Result<State> {
    let data = match state {
        State::Initial(..) => return Err(eyre!("You must SelectIssue first.")),
        State::IssueSelected(data) => data,
    };
    let repo = Repository::open(".").wrap_err("Could not find Git repo in this directory")?;
    let new_branch_name = branch_name_from_issue(&data.issue);
    let branches = get_all_branches(&repo)?;

    if let Some(existing) = find_branch(&new_branch_name, &branches) {
        println!(
            "Found existing branch named {}, switching to it.",
            new_branch_name
        );
        switch_to_branch(&repo, existing)?;
        return Ok(State::IssueSelected(data));
    }

    println!("Createing a new branch called {}", new_branch_name);
    let branch = select_branch(branches, "Which branch do you want to base off of?")?;
    let new_branch = create_branch(&repo, &new_branch_name, &branch)?;
    switch_to_branch(&repo, &new_branch)?;
    Ok(State::IssueSelected(data))
}

fn create_branch<'repo>(
    repo: &'repo Repository,
    name: &str,
    branch: &Branch,
) -> Result<Branch<'repo>> {
    repo.branch(name, &branch.get().peel_to_commit()?, false)
        .wrap_err_with(|| format!("Failed to create new branch {}", name))
}

fn select_branch<'repo>(branches: Vec<Branch<'repo>>, prompt: &str) -> Result<Branch<'repo>> {
    let branch_names: Vec<&str> = branches
        .iter()
        .map(|b| b.name())
        .filter_map(|name| name.ok())
        .filter_map(|name| name)
        .collect();

    let base_branch_name = select(branch_names, prompt)
        .wrap_err("failed to select branch")?
        .to_owned();

    branches
        .into_iter()
        .find(|b| b.name().ok() == Some(Some(&base_branch_name)))
        .wrap_err("failed to select branch")
}

fn switch_to_branch(repo: &Repository, branch: &Branch) -> Result<()> {
    let ref_name = branch
        .get()
        .name()
        .ok_or_else(|| eyre!("problem checking out branch, could not parse name"))?;
    repo.set_head(ref_name)
        .wrap_err_with(|| format!("Found branch {} but could not switch to it.", ref_name))
}

fn find_branch<'vec, 'repo>(
    name: &str,
    branches: &'vec Vec<Branch<'repo>>,
) -> Option<&'vec Branch<'repo>> {
    branches.iter().find(|b| b.name().ok() == Some(Some(name)))
}

fn get_all_branches(repo: &Repository) -> Result<Vec<Branch>> {
    Ok(repo
        .branches(None)
        .wrap_err("Could not list branches")?
        .into_iter()
        .filter_map(|b| b.ok())
        .map(|(b, _)| b)
        .collect())
}

fn branch_name_from_issue(issue: &Issue) -> String {
    format!("{}-{}", issue.key, issue.summary.to_ascii_lowercase()).replace(" ", "-")
}
