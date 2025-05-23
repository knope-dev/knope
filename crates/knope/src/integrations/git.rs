use std::{
    collections::{HashMap, HashSet, VecDeque},
    env::current_dir,
    str::FromStr,
};

use git2::{Branch, BranchType, IndexAddOption, Oid, Repository, build::CheckoutBuilder};
use itertools::Itertools;
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use tracing::{debug, info};

use crate::{RunType, fs, prompt, prompt::select, state, state::State, step::issues::Issue};

/// Based on the selected issue, either checks out an existing branch matching the name or creates
/// a new one, prompting for which branch to base it on.
pub(crate) fn switch_branches(state: RunType<State>) -> Result<RunType<State>, Error> {
    let (state, dry_run) = match state {
        RunType::DryRun(state) => (state, true),
        RunType::Real(state) => (state, false),
    };
    let issue = match &state.issue {
        state::Issue::Initial => return Err(ErrorKind::NoIssueSelected.into()),
        state::Issue::Selected(issue) => issue,
    };
    let new_branch_name = branch_name_from_issue(issue);
    if dry_run {
        info!("Would switch to or create a branch named {new_branch_name}");
        return Ok(RunType::DryRun(state));
    }

    let repo = Repository::open(".").map_err(ErrorKind::OpenRepo)?;
    let branches = get_all_branches(&repo)?;

    if let Ok(existing) = repo.find_branch(&new_branch_name, BranchType::Local) {
        info!("Found existing branch named {new_branch_name}, switching to it.");
        switch_to_branch(&repo, &existing)?;
    } else {
        info!("Creating a new branch called {new_branch_name}");
        let branch = select_branch(branches, "Which branch do you want to base off of?")?;
        let new_branch = create_branch(&repo, &new_branch_name, &branch)?;
        switch_to_branch(&repo, &new_branch)?;
    }

    Ok(RunType::Real(state))
}

#[derive(Debug, Diagnostic, thiserror::Error)]
#[error(transparent)]
#[diagnostic(transparent)]
pub(crate) struct Error(Box<ErrorKind>);

impl<T: Into<ErrorKind>> From<T> for Error {
    fn from(kind: T) -> Self {
        Self(Box::new(kind.into()))
    }
}

#[derive(Debug, Diagnostic, thiserror::Error)]
enum ErrorKind {
    #[error("Could not determine current directory: {0}")]
    CurrentDirectory(std::io::Error),
    #[error("Could not open Git repository: {0}")]
    #[diagnostic(
        code(git::open_repo),
        help("Make sure you are in a Git repository and that you have permission to access it.")
    )]
    OpenRepo(#[source] git2::Error),
    #[error("No issue selected")]
    #[diagnostic(
        code(git::no_issue_selected),
        help(
            "Switching branches requires selecting an issue first with SelectGitHubIssue or SelectJiraIssue"
        )
    )]
    NoIssueSelected,
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
    #[error("Unknown Git error: {0}")]
    #[diagnostic(
        code(git::libgit2),
        help(
            "Something went wrong when interacting with Git that we don't have an explanation for. \
                    Maybe try performing the operation manually?"
        )
    )]
    Git(#[from] git2::Error),
    #[error("Not on the tip of a Git branch.")]
    #[diagnostic(
        code(git::not_a_branch),
        help("In order to run this step, you need to be on the very tip of a Git branch.")
    )]
    NotOnAGitBranch,
    #[error("Bad branch name")]
    #[diagnostic(
        code(git::bad_branch_name),
        help("The branch name was not formatted correctly."),
        url("https://knope.tech/reference/config-file/steps/select-issue-from-branch/")
    )]
    BadGitBranchName,
    #[error("Uncommitted changes")]
    #[diagnostic(
        code(git::uncommitted_changes),
        help("You need to commit your changes before running this step."),
        url("https://knope.tech/reference/config-file/steps/switch-branches/")
    )]
    UncommittedChanges,
    #[error("Could not complete checkout")]
    #[diagnostic(
        code(git::incomplete_checkout),
        help(
            "Switching branches failed, but HEAD was changed. You probably want to git switch back \
                to the branch you were on."
        )
    )]
    IncompleteCheckout(#[source] git2::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Prompt(#[from] prompt::Error),
    #[error("Could not determine Git committer to commit changes: {0}")]
    #[diagnostic(
        code(git::no_committer),
        help(
            "We couldn't determine who to commit the changes as. Please set the `user.name` and \
                `user.email` Git config options."
        )
    )]
    NoCommitter(git2::Error),
}

/// Rebase the current branch onto the selected one.
pub(crate) fn rebase_branch(to: &RunType<String>) -> Result<(), Error> {
    let to = match to {
        RunType::DryRun(to) => {
            info!("Would rebase current branch onto {to}");
            return Ok(());
        }
        RunType::Real(to) => to,
    };

    let repo = Repository::open(".").map_err(ErrorKind::OpenRepo)?;
    let head = repo.head()?;

    let target_branch = repo.find_branch(to, BranchType::Local)?;
    let target = repo.reference_to_annotated_commit(target_branch.get())?;
    let source = repo.reference_to_annotated_commit(&head)?;
    repo.rebase(Some(&target), None, Some(&source), None)?
        .finish(None)?;

    info!("Rebased current branch onto {to}");
    switch_to_branch(&repo, &target_branch)?;
    info!("Switched to branch {to}, don't forget to push!");
    Ok(())
}

pub(crate) fn select_issue_from_current_branch(
    state: RunType<State>,
) -> Result<RunType<State>, Error> {
    match state {
        RunType::DryRun(mut state) => {
            info!("Would attempt to parse current branch name to select current issue");
            state.issue = state::Issue::Selected(Issue {
                key: String::from("123"),
                summary: String::from("Fake Issue"),
            });
            Ok(RunType::DryRun(state))
        }
        RunType::Real(mut state) => {
            let current_branch = current_branch()?;
            let issue = select_issue_from_branch_name(&current_branch)?;
            state.issue = state::Issue::Selected(issue);
            Ok(RunType::Real(state))
        }
    }
}

pub(crate) fn current_branch() -> Result<String, Error> {
    let repo = Repository::open(".").map_err(ErrorKind::OpenRepo)?;
    let head = repo.head()?;
    let ref_name = head.name().ok_or(ErrorKind::NotOnAGitBranch)?;
    Ok(ref_name.to_owned())
}

/// Get the first remote of the Git repo, if any.
pub(crate) fn get_first_remote() -> Option<String> {
    let repo = Repository::open(".").ok()?;
    let remotes = repo.remotes().ok()?;
    let remote_name = remotes.get(0)?;
    repo.find_remote(remote_name)
        .ok()
        .and_then(|remote| remote.url().map(String::from))
}

fn select_issue_from_branch_name(ref_name: &str) -> Result<Issue, Error> {
    let mut parts: VecDeque<&str> = ref_name.split('-').collect();

    let issue_key = parts.pop_front().ok_or(ErrorKind::BadGitBranchName)?;
    if let Ok(github_issue) = usize::from_str(issue_key) {
        info!("Auto-selecting issue {github_issue} from ref {ref_name}");
        return Ok(Issue {
            key: github_issue.to_string(),
            summary: parts.iter().join("-"),
        });
    }
    let project_key = issue_key;
    let issue_number = parts
        .pop_front()
        .map(usize::from_str)
        .ok_or(ErrorKind::BadGitBranchName)?
        .or(Err(ErrorKind::BadGitBranchName))?;
    let jira_issue = format!("{project_key}-{issue_number}");
    info!("Auto-selecting issue {jira_issue} from ref {ref_name}");
    Ok(Issue {
        key: jira_issue,
        summary: parts.iter().join("-"),
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
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
                summary: "some-summary".to_string(),
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
                summary: "some-summary".to_string(),
            }
        );
    }

    #[test]
    fn no_number() {
        let result = select_issue_from_branch_name("some-summary");

        assert!(result.is_err());
    }
}

fn create_branch<'repo>(
    repo: &'repo Repository,
    name: &str,
    branch: &Branch,
) -> Result<Branch<'repo>, Error> {
    repo.branch(name, &branch.get().peel_to_commit()?, false)
        .map_err(Error::from)
}

fn select_branch<'repo>(
    branches: Vec<Branch<'repo>>,
    prompt: &str,
) -> Result<Branch<'repo>, Error> {
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
        .ok_or(ErrorKind::BadGitBranchName.into())
}

fn switch_to_branch(repo: &Repository, branch: &Branch) -> Result<(), Error> {
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
        return Err(ErrorKind::UncommittedChanges.into());
    }
    let ref_name = branch
        .get()
        .name()
        .ok_or(Error::from(ErrorKind::BadGitBranchName))?;
    repo.set_head(ref_name)?;
    repo.checkout_head(Some(CheckoutBuilder::new().force()))
        .map_err(ErrorKind::IncompleteCheckout)?;
    Ok(())
}

fn get_all_branches(repo: &Repository) -> Result<Vec<Branch>, Error> {
    Ok(repo
        .branches(Some(BranchType::Local))?
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

#[cfg(test)]
mod test_branch_name_from_issue {
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

/// Add some files to Git to be committed later.
pub(crate) fn add_files(file_names: &[RelativePathBuf]) -> Result<(), Error> {
    if file_names.is_empty() {
        return Ok(());
    }
    let repo = Repository::open(".").map_err(ErrorKind::OpenRepo)?;
    let mut index = repo.index()?;
    index.add_all(
        file_names.iter().map(|rel_path| rel_path.to_path("")),
        IndexAddOption::DEFAULT,
        None,
    )?;
    index.write().map_err(Error::from)
}

/// Find every commit that appears only _after_ a specific tag.
///
/// This builds a complete set of every commit in the repository, because branching and merging
/// means that there could be paths which jump _behind_ the target tag... and we want to exclude
/// those as well. There's probably a way to optimize performance with some cool graph magic
/// eventually, but this is good enough for now.
pub(crate) fn get_commit_messages_after_tag(tag: &str) -> Result<Vec<String>, Error> {
    let repo = Repository::open(".")?;

    let tag_ref_name = format!("refs/tags/{tag}");
    let tag_ref = repo.find_reference(&tag_ref_name).ok();

    if tag_ref.is_some() {
        debug!("Using commits since tag {tag}");
    } else {
        debug!("Tag {tag} not found, using ALL commits");
    }

    // Get the commit that the tag points to (if it exists)
    let tag_commit_id = tag_ref
        .and_then(|reference| reference.peel_to_commit().ok())
        .map(|commit| commit.id());

    // Create a set of all commit IDs to exclude
    let mut commits_to_exclude = HashSet::new();

    // If we found the tag, add all its ancestors to the exclude set
    if let Some(commit_id) = tag_commit_id {
        let mut revwalk = repo.revwalk()?;
        revwalk.push(commit_id)?;

        for ancestor_id in revwalk.filter_map(Result::ok) {
            commits_to_exclude.insert(ancestor_id);
        }
    }

    // Get all commits from HEAD that aren't in the exclude set
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    let mut commits = Vec::new();
    for oid in revwalk.filter_map(Result::ok) {
        if !commits_to_exclude.contains(&oid) {
            if let Ok(commit) = repo.find_commit(oid) {
                if let Some(message) = commit.message().map(String::from) {
                    commits.push(message);
                }
            }
        }
    }

    // Reverse to get chronological order (oldest first)
    commits.reverse();

    Ok(commits)
}

pub(crate) fn create_tag(name: RunType<&str>) -> Result<(), Error> {
    match name {
        RunType::DryRun(name) => {
            info!("Would create Git tag {name}");
            Ok(())
        }
        RunType::Real(name) => {
            let repo = Repository::open(current_dir().map_err(ErrorKind::CurrentDirectory)?)?;

            let head = repo.head()?;
            let head_commit = head.peel_to_commit()?;

            let signature = repo.signature().map_err(ErrorKind::NoCommitter)?;

            repo.tag(name, &head_commit.into_object(), &signature, "", false)?;

            Ok(())
        }
    }
}

/// Get all tags on the current branch.
pub(crate) fn all_tags_on_branch() -> Result<Vec<String>, Error> {
    let repo = Repository::open(current_dir().map_err(ErrorKind::CurrentDirectory)?)?;
    let mut all_tags: HashMap<Oid, Vec<String>> = HashMap::new();
    for reference in repo.references()?.filter_map(Result::ok) {
        let Some(name) = reference.name() else {
            continue;
        };
        if !name.starts_with("refs/tags/") {
            continue;
        }
        let name = name.trim_start_matches("refs/tags/");
        if let Ok(target) = reference.peel_to_commit() {
            all_tags
                .entry(target.id())
                .or_default()
                .push(name.to_string());
        }
    }

    let mut tags: Vec<String> = Vec::with_capacity(all_tags.len());
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    for commit_id in revwalk.filter_map(Result::ok) {
        if let Some(tag) = all_tags.remove(&commit_id) {
            tags.extend(tag);
        }
    }
    if !all_tags.is_empty() {
        debug!(
            "Skipping relevant tags that are not on the current branch: {tags}",
            tags = all_tags.values().flatten().join(", ")
        );
    }
    Ok(tags)
}
