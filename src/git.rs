use color_eyre::eyre::{eyre, ContextCompat, Result, WrapErr};
use git2::{Branch, BranchType, DescribeFormatOptions, DescribeOptions, Repository};
use regex::Regex;

use crate::issues::Issue;
use crate::prompt::select;
use crate::state::{Initial, IssueSelected, State};
use git2::build::CheckoutBuilder;

/// Based on the selected issue, either checks out an existing branch matching the name or creates
/// a new one, prompting for which branch to base it on.
pub(crate) fn switch_branches(state: State) -> Result<State> {
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
        switch_to_branch(&repo, &existing)?;
        return Ok(State::IssueSelected(data));
    }

    println!("Creating a new branch called {}", new_branch_name);
    let branch = select_branch(branches, "Which branch do you want to base off of?")?;
    let new_branch = create_branch(&repo, &new_branch_name, &branch)?;
    switch_to_branch(&repo, &new_branch)?;
    Ok(State::IssueSelected(data))
}

/// Rebase the current branch onto the selected one.
pub(crate) fn rebase_branch(state: State, to: &str) -> Result<State> {
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
        .find_branch(to, BranchType::Local)
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
    switch_to_branch(&repo, &target_branch)?;
    println!("Switched to branch {}, don't forget to push!", to);

    Ok(State::IssueSelected(data))
}

pub(crate) fn select_issue_from_current_branch(state: State) -> Result<State> {
    let state_data = match state {
        State::IssueSelected(IssueSelected {
            jira_config,
            github_state,
            github_config,
            ..
        }) => Initial {
            jira_config,
            github_state,
            github_config,
        },
        State::Initial(data) => data,
    };
    let repo = Repository::open(".").wrap_err("Could not find Git repo in this directory")?;
    let head = repo.head().wrap_err("Could not resolve Repo HEAD")?;
    let ref_name = head.name().ok_or_else(|| {
        eyre!("Could not get a name for current HEAD. Are you at the tip of a branch?")
    })?;
    let state_data = select_issue_from_branch_name(state_data, ref_name)?;
    Ok(State::IssueSelected(state_data))
}

fn select_issue_from_branch_name(data: Initial, ref_name: &str) -> Result<IssueSelected> {
    let re = Regex::new("((?:[A-Z]+-)?[0-9]+)-(.*)").unwrap();
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
        github_state: data.github_state,
        github_config: data.github_config,
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
        .map(Branch::name)
        .filter_map(Result::ok)
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
    let statuses = repo.statuses(None).wrap_err("Could not get Git statuses")?;
    let uncommitted_changes = statuses.iter().any(|status| {
        if let Ok(path) = String::from_utf8(Vec::from(status.path_bytes())) {
            if matches!(repo.status_should_ignore(path.as_ref()), Ok(false)) {
                return true;
            }
        }
        false
    });
    if uncommitted_changes {
        return Err(eyre!(
            "Cannot switch branches if you have uncommitted changes. Stash, then try again."
        ));
    }
    let ref_name = branch
        .get()
        .name()
        .ok_or_else(|| eyre!("problem checking out branch, could not parse name"))?;
    repo.set_head(ref_name)
        .wrap_err_with(|| format!("Found branch {} but could not switch to it.", ref_name))?;
    repo.checkout_head(Some(CheckoutBuilder::new().force()))
        .wrap_err(
            "Switching branches failed, but HEAD was changed. You probably want to git switch back to the branch you were on",
        )?;
    Ok(())
}

fn get_all_branches(repo: &Repository) -> Result<Vec<Branch>> {
    Ok(repo
        .branches(Some(BranchType::Local))
        .wrap_err("Could not list branches")?
        .into_iter()
        .filter_map(|value| {
            if let Ok((b, _)) = value {
                Some(b)
            } else {
                None
            }
        })
        .collect())
}

pub(crate) fn branch_name_from_issue(issue: &Issue) -> String {
    format!("{}-{}", issue.key, issue.summary.to_ascii_lowercase()).replace(" ", "-")
}

fn get_last_tag_name(repo: &Repository) -> Result<String> {
    repo.describe(&DescribeOptions::new().describe_tags())
        .wrap_err("Could not describe project, are there any tags?")?
        .format(Some(DescribeFormatOptions::new().abbreviated_size(0)))
        .wrap_err("Could not format description into tag.")
}

pub(crate) fn get_commit_messages_after_last_tag() -> Result<Vec<String>> {
    let repo =
        Repository::open(".").wrap_err("Could not open Git repository in working directory.")?;
    let tag_name = get_last_tag_name(&repo)?;
    let tag_ref = repo
        .find_reference(&format!("refs/tags/{}", tag_name))
        .wrap_err_with(|| format!("Could not find tag {}", tag_name))?;
    let tag_oid = tag_ref
        .target()
        .ok_or_else(|| eyre!("Could not find object described by tag {}", tag_name))?;
    let mut revwalk = repo.revwalk()?;
    revwalk
        .push_head()
        .wrap_err("Could not start walking history from HEAD")?;
    let messages: Vec<String> = revwalk
        .into_iter()
        .filter_map(std::result::Result::ok)
        .take_while(|oid| oid != &tag_oid)
        .filter_map(|oid| {
            if let Ok(commit) = repo.find_commit(oid) {
                Some(commit.message().unwrap_or("").to_string())
            } else {
                None
            }
        })
        .collect();
    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_name_from_issue() {
        let issue = Issue {
            key: "FLOW-5".to_string(),
            summary: "A test issue".to_string(),
        };
        let branch_name = super::branch_name_from_issue(&issue);
        assert_eq!(&branch_name, "FLOW-5-a-test-issue");
    }
}

#[cfg(test)]
mod test_select_issue_from_branch_name {
    use super::*;
    use crate::state::GitHub;

    #[test]
    fn jira_style() {
        let data = select_issue_from_branch_name(
            Initial {
                jira_config: None,
                github_state: GitHub::New,
                github_config: None,
            },
            "ABC-123-some-summary",
        )
        .expect("Failed to parse branch name");

        assert_eq!(
            data.issue,
            Issue {
                key: "ABC-123".to_string(),
                summary: "some-summary".to_string()
            }
        )
    }

    #[test]
    fn github_style() {
        let data = select_issue_from_branch_name(
            Initial {
                jira_config: None,
                github_state: GitHub::New,
                github_config: None,
            },
            "123-some-summary",
        )
        .expect("Failed to parse branch name");

        assert_eq!(
            data.issue,
            Issue {
                key: "123".to_string(),
                summary: "some-summary".to_string()
            }
        )
    }

    #[test]
    fn no_number() {
        let result = select_issue_from_branch_name(
            Initial {
                jira_config: None,
                github_state: GitHub::New,
                github_config: None,
            },
            "some-summary",
        );

        assert!(result.is_err())
    }
}
