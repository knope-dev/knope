use std::str::FromStr;

use git2::build::CheckoutBuilder;
use git2::{Branch, BranchType, DescribeFormatOptions, DescribeOptions, Repository};

use crate::issues::Issue;
use crate::prompt::select;
use crate::state;
use crate::step::StepError;
use crate::RunType;

/// Based on the selected issue, either checks out an existing branch matching the name or creates
/// a new one, prompting for which branch to base it on.
pub(crate) fn switch_branches(run_type: RunType) -> Result<RunType, StepError> {
    let (state, dry_run_stdout) = run_type.decompose();
    let issue = match &state.issue {
        state::Issue::Initial => return Err(StepError::NoIssueSelected),
        state::Issue::Selected(issue) => issue,
    };
    let new_branch_name = branch_name_from_issue(issue);
    if let Some(mut stdout) = dry_run_stdout {
        writeln!(
            stdout,
            "Would switch to or create a branch named {}",
            new_branch_name
        )?;
        return Ok(RunType::DryRun { state, stdout });
    }

    let repo = Repository::open(".").map_err(|_| StepError::NotAGitRepo)?;
    let branches = get_all_branches(&repo)?;

    if let Ok(existing) = repo.find_branch(&new_branch_name, BranchType::Local) {
        println!(
            "Found existing branch named {}, switching to it.",
            new_branch_name
        );
        switch_to_branch(&repo, &existing)?;
    } else {
        println!("Creating a new branch called {}", new_branch_name);
        let branch = select_branch(branches, "Which branch do you want to base off of?")?;
        let new_branch = create_branch(&repo, &new_branch_name, &branch)?;
        switch_to_branch(&repo, &new_branch)?;
    }

    Ok(RunType::Real(state))
}

/// Rebase the current branch onto the selected one.
pub(crate) fn rebase_branch(to: &str, mut run_type: RunType) -> Result<RunType, StepError> {
    if let RunType::DryRun { stdout, .. } = &mut run_type {
        writeln!(stdout, "Would rebase current branch onto {}", to)?;
        return Ok(run_type);
    }

    let repo = Repository::open(".").map_err(|_| StepError::NotAGitRepo)?;
    let head = repo.head()?;

    let target_branch = repo.find_branch(to, BranchType::Local)?;
    let target = repo.reference_to_annotated_commit(target_branch.get())?;
    let source = repo.reference_to_annotated_commit(&head)?;
    repo.rebase(Some(&target), None, Some(&source), None)?
        .finish(None)?;

    println!("Rebased current branch onto {}", to);
    switch_to_branch(&repo, &target_branch)?;
    println!("Switched to branch {}, don't forget to push!", to);
    Ok(run_type)
}

pub(crate) fn select_issue_from_current_branch(run_type: RunType) -> Result<RunType, StepError> {
    match run_type {
        RunType::DryRun {
            mut state,
            mut stdout,
        } => {
            writeln!(
                stdout,
                "Would attempt to parse current branch name to select current issue"
            )?;
            state.issue = state::Issue::Selected(Issue {
                key: String::from("123"),
                summary: String::from("Fake Issue"),
            });
            Ok(RunType::DryRun { state, stdout })
        }
        RunType::Real(mut state) => {
            let repo = Repository::open(".").map_err(|_| StepError::NotAGitRepo)?;
            let head = repo.head()?;
            let ref_name = head.name().ok_or(StepError::NotOnAGitBranch)?;
            let issue = select_issue_from_branch_name(ref_name)?;
            state.issue = state::Issue::Selected(issue);
            Ok(RunType::Real(state))
        }
    }
}

fn select_issue_from_branch_name(ref_name: &str) -> Result<Issue, StepError> {
    let parts: Vec<&str> = ref_name.split('-').collect();

    let (key, summary) = if !parts.is_empty() && usize::from_str(parts[0]).is_ok() {
        // GitHub style, like 42-some-description for issue #42
        Ok((parts[0].to_string(), parts[1..].join("-")))
    } else if parts.len() >= 2 && usize::from_str(parts[1]).is_ok() {
        // Jira style, like PROJ-123-something-else where PROJ-123 is the issue key
        Ok((parts[0..2].join("-"), parts[2..].join("-")))
    } else {
        Err(StepError::BadGitBranchName)
    }?;

    println!("Auto-selecting issue {} from ref {}", &key, ref_name);
    Ok(Issue { key, summary })
}

fn create_branch<'repo>(
    repo: &'repo Repository,
    name: &str,
    branch: &Branch,
) -> Result<Branch<'repo>, StepError> {
    repo.branch(name, &branch.get().peel_to_commit()?, false)
        .map_err(StepError::from)
}

fn select_branch<'repo>(
    branches: Vec<Branch<'repo>>,
    prompt: &str,
) -> Result<Branch<'repo>, StepError> {
    let branch_names: Vec<&str> = branches
        .iter()
        .map(Branch::name)
        .filter_map(Result::ok)
        .flatten()
        .collect();

    let base_branch_name = select(branch_names, prompt)?.to_owned();

    branches
        .into_iter()
        .find(|b| b.name().ok() == Some(Some(&base_branch_name)))
        .ok_or(StepError::BadGitBranchName)
}

fn switch_to_branch(repo: &Repository, branch: &Branch) -> Result<(), StepError> {
    let statuses = repo.statuses(None)?;
    let uncommitted_changes = statuses.iter().any(|status| {
        if let Ok(path) = String::from_utf8(Vec::from(status.path_bytes())) {
            if matches!(repo.status_should_ignore(path.as_ref()), Ok(false)) {
                return true;
            }
        }
        false
    });
    if uncommitted_changes {
        return Err(StepError::UncommittedChanges);
    }
    let ref_name = branch.get().name().ok_or(StepError::BadGitBranchName)?;
    repo.set_head(ref_name)?;
    repo.checkout_head(Some(CheckoutBuilder::new().force()))
        .map_err(StepError::IncompleteCheckout)?;
    Ok(())
}

fn get_all_branches(repo: &Repository) -> Result<Vec<Branch>, StepError> {
    Ok(repo
        .branches(Some(BranchType::Local))?
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
    format!("{}-{}", issue.key, issue.summary.to_ascii_lowercase()).replace(' ', "-")
}

fn get_last_tag_name(repo: &Repository) -> Result<String, StepError> {
    repo.describe(DescribeOptions::new().describe_tags())
        .map_err(StepError::ListTagsError)?
        .format(Some(DescribeFormatOptions::new().abbreviated_size(0)))
        .map_err(StepError::from)
}

pub(crate) fn get_commit_messages_after_last_tag() -> Result<Vec<String>, StepError> {
    let repo = Repository::open(".").map_err(|_| StepError::NotAGitRepo)?;
    let tag_name = get_last_tag_name(&repo)?;
    let tag_ref = repo.find_reference(&format!("refs/tags/{}", tag_name))?;
    let tag_oid = tag_ref.target().ok_or(StepError::GitError(None))?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
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

    #[test]
    fn jira_style() {
        let issue = select_issue_from_branch_name("ABC-123-some-summary")
            .expect("Failed to parse branch name");

        assert_eq!(
            issue,
            Issue {
                key: "ABC-123".to_string(),
                summary: "some-summary".to_string()
            }
        );
    }

    #[test]
    fn github_style() {
        let issue =
            select_issue_from_branch_name("123-some-summary").expect("Failed to parse branch name");

        assert_eq!(
            issue,
            Issue {
                key: "123".to_string(),
                summary: "some-summary".to_string()
            }
        );
    }

    #[test]
    fn no_number() {
        let result = select_issue_from_branch_name("some-summary");

        assert!(result.is_err());
    }
}
