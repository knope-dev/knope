use color_eyre::eyre::{eyre, ContextCompat, Result, WrapErr};
use git2::{Branch, BranchType, Repository};
use regex::Regex;

use crate::prompt::select;
use crate::state::{Initial, Issue, IssueSelected, State};

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

    if let Ok(existing) = repo.find_branch(&new_branch_name, BranchType::Local) {
        println!(
            "Found existing branch named {}, switching to it.",
            new_branch_name
        );
        switch_to_branch(&repo, existing)?;
        return Ok(State::IssueSelected(data));
    }

    println!("Creating a new branch called {}", new_branch_name);
    let branch = select_branch(branches, "Which branch do you want to base off of?")?;
    let new_branch = create_branch(&repo, &new_branch_name, &branch)?;
    switch_to_branch(&repo, new_branch)?;
    Ok(State::IssueSelected(data))
}

/// Rebase the current branch onto the selected one.
pub fn rebase_branch(state: State, to: String) -> Result<State> {
    let repo = Repository::open(".").wrap_err("Could not find Git repo in this directory")?;
    let head = repo.head().wrap_err("Could not resolve Repo HEAD")?;
    let ref_name = head.name().ok_or_else(|| {
        eyre!("Could not get a name for current HEAD. Are you at the tip of a branch?")
    })?;
    let data = match state {
        State::Initial(data) => select_issue_from_branch_name(data, ref_name)?,
        State::IssueSelected(data) => data,
    };

    let target_branch = repo
        .find_branch(&to, BranchType::Local)
        .wrap_err_with(|| format!("Could not find target branch {}, is it local?", to))?;
    let target = repo
        .reference_to_annotated_commit(target_branch.get())
        .wrap_err("Could not retrieve annotated commit from target to rebase")?;
    let source = repo
        .reference_to_annotated_commit(&head)
        .wrap_err("Could not retrieve annotated commit from source to rebase")?;
    repo.rebase(Some(&target), None, Some(&source), None)
        .wrap_err("Failed to start rebase")?
        .finish(None)
        .wrap_err("Could not complete rebase")?;

    println!("Rebased current branch onto {}", to);
    switch_to_branch(&repo, target_branch)?;
    println!("Switched to branch {}, don't forget to push!", to);

    Ok(State::IssueSelected(data))
}

fn select_issue_from_branch_name(data: Initial, ref_name: &str) -> Result<IssueSelected> {
    let re = Regex::new("([A-Z]+-[0-9]+)(.*)").unwrap();
    let caps = re.captures(ref_name).ok_or_else(|| {
        eyre!(
            "Current ref {} is not in the right format. Was it created with Flow?",
            ref_name
        )
    })?;
    let key = caps
        .get(1)
        .ok_or_else(|| eyre!("Could not determine Jira issue key from ref {}", ref_name))?
        .as_str()
        .to_owned();
    let summary = caps
        .get(2)
        .ok_or_else(|| {
            eyre!(
                "Could not determine Jira issue summary from ref {}",
                ref_name
            )
        })?
        .as_str()
        .to_owned();
    println!("Auto-selecting issue {} from ref {}", &key, ref_name);
    Ok(IssueSelected {
        jira_config: data.jira_config,
        issue: Issue { key, summary },
    })
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

fn switch_to_branch(repo: &Repository, branch: Branch) -> Result<()> {
    let ref_name = branch
        .get()
        .name()
        .ok_or_else(|| eyre!("problem checking out branch, could not parse name"))?;
    repo.set_head(ref_name)
        .wrap_err_with(|| format!("Found branch {} but could not switch to it.", ref_name))
}

fn get_all_branches(repo: &Repository) -> Result<Vec<Branch>> {
    Ok(repo
        .branches(Some(BranchType::Local))
        .wrap_err("Could not list branches")?
        .into_iter()
        .filter_map(|b| b.ok())
        .map(|(b, _)| b)
        .collect())
}

fn branch_name_from_issue(issue: &Issue) -> String {
    format!("{}-{}", issue.key, issue.summary.to_ascii_lowercase()).replace(" ", "-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn branch_name_from_issue() {
        let issue = Issue {
            key: "FLOW-5".to_string(),
            summary: "A test issue".to_string(),
        };
        let branch_name = super::branch_name_from_issue(&issue);
        assert_eq!(&branch_name, "FLOW-5-a-test-issue");
    }

    #[test]
    fn get_all_branches() {
        let output = String::from_utf8(
            Command::new("git")
                .arg("branch")
                .output()
                .expect("Failed to execute command")
                .stdout,
        )
        .expect("Output was not UTF-8");
        let expected = output
            .split("\n")
            .filter(|b| !b.is_empty())
            .map(|b| b.replace(" ", "").replace("*", ""))
            .collect::<Vec<_>>();

        let repo = Repository::open(".").expect("Could not open repo");
        let actual = super::get_all_branches(&repo)
            .expect("Could not list branches")
            .into_iter()
            .map(|branch| {
                branch
                    .name()
                    .expect("Could not get name")
                    .expect("No name")
                    .to_string()
            })
            .collect::<Vec<_>>();

        assert_eq!(expected, actual);
    }
}
