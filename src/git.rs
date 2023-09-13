use std::{
    collections::{HashMap, HashSet, VecDeque},
    env::current_dir,
    path::PathBuf,
    str::FromStr,
};

use git2::{build::CheckoutBuilder, Branch, BranchType, IndexAddOption, Repository};
use gix::{object::Kind, refs::transaction::PreviousValue, ObjectId};
use itertools::Itertools;
use log::error;
use miette::Diagnostic;

use crate::{
    dry_run::DryRun,
    fs,
    issues::Issue,
    prompt,
    prompt::select,
    releases::{semver::Version, CurrentVersions},
    state,
    workflow::Verbose,
    RunType,
};

/// Based on the selected issue, either checks out an existing branch matching the name or creates
/// a new one, prompting for which branch to base it on.
pub(crate) fn switch_branches(run_type: RunType) -> Result<RunType, Error> {
    let (state, dry_run_stdout) = run_type.decompose();
    let issue = match &state.issue {
        state::Issue::Initial => return Err(ErrorKind::NoIssueSelected.into()),
        state::Issue::Selected(issue) => issue,
    };
    let new_branch_name = branch_name_from_issue(issue);
    if let Some(mut stdout) = dry_run_stdout {
        writeln!(
            stdout,
            "Would switch to or create a branch named {new_branch_name}"
        )
        .map_err(fs::Error::Stdout)?;
        return Ok(RunType::DryRun { state, stdout });
    }

    let repo = Repository::open(".").map_err(ErrorKind::OpenRepo)?;
    let branches = get_all_branches(&repo)?;

    if let Ok(existing) = repo.find_branch(&new_branch_name, BranchType::Local) {
        println!("Found existing branch named {new_branch_name}, switching to it.");
        switch_to_branch(&repo, &existing)?;
    } else {
        println!("Creating a new branch called {new_branch_name}");
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
        help("Switching branches requires selecting an issue first with SelectGitHubIssue or SelectJiraIssue")
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
        url("https://knope-dev.github.io/knope/config/step/SelectIssueFromBranch.html")
    )]
    BadGitBranchName,
    #[error("Uncommitted changes")]
    #[diagnostic(
        code(git::uncommitted_changes),
        help("You need to commit your changes before running this step."),
        url("https://knope-dev.github.io/knope/config/step/SwitchBranches.html")
    )]
    UncommittedChanges,
    #[error("Could not complete checkout")]
    #[diagnostic(
        code(git::incomplete_checkout),
        help("Switching branches failed, but HEAD was changed. You probably want to git switch back \
                to the branch you were on."),
    )]
    IncompleteCheckout(#[source] git2::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Prompt(#[from] prompt::Error),
    #[error("Could not open Git repository: {0}")]
    #[diagnostic(
        code(git::open_git_repo),
        help("Please check that the current directory is a Git repository.")
    )]
    OpenGitRepo(#[from] gix::open::Error),
    #[error("Could not get Git references to parse tags: {0}")]
    GitReferences(#[from] gix::reference::iter::Error),
    #[error("Could not get Git tags: {0}")]
    Tags(#[from] gix::reference::iter::init::Error),
    #[error("Could not find head commit: {0}")]
    HeadCommit(#[from] gix::reference::head_commit::Error),
    #[error("Could not determine Git committer to commit changes")]
    #[diagnostic(
        code(git::no_committer),
        help(
            "We couldn't determine who to commit the changes as. Please set the `user.name` and \
                `user.email` Git config options."
        )
    )]
    NoCommitter,
    #[error("Could not create a tag: {0}")]
    #[diagnostic(
        code(git::tag_failed),
        help("A Git tag could not be created for the release.")
    )]
    CreateTagError(#[from] gix::tag::Error),
    #[error("Could not find reference {reference}: {source}")]
    #[diagnostic(
        code(releases::git::find_reference),
        help("Please check that the reference exists.")
    )]
    FindReference {
        reference: String,
        #[source]
        source: gix::reference::find::existing::Error,
    },
    #[error("Could not peel oid: {0}")]
    #[diagnostic(
        code(releases::git::peel_oid),
        help("Please check that the reference exists.")
    )]
    PeelOid(#[from] gix::reference::peel::Error),
    #[error("Could not walk commits back from HEAD: {0}")]
    RevisionWalk(#[from] gix::revision::walk::Error),
}

/// Rebase the current branch onto the selected one.
pub(crate) fn rebase_branch(to: &str, mut run_type: RunType) -> Result<RunType, Error> {
    if let RunType::DryRun { stdout, .. } = &mut run_type {
        writeln!(stdout, "Would rebase current branch onto {to}").map_err(fs::Error::Stdout)?;
        return Ok(run_type);
    }

    let repo = Repository::open(".").map_err(ErrorKind::OpenRepo)?;
    let head = repo.head()?;

    let target_branch = repo.find_branch(to, BranchType::Local)?;
    let target = repo.reference_to_annotated_commit(target_branch.get())?;
    let source = repo.reference_to_annotated_commit(&head)?;
    repo.rebase(Some(&target), None, Some(&source), None)?
        .finish(None)?;

    println!("Rebased current branch onto {to}");
    switch_to_branch(&repo, &target_branch)?;
    println!("Switched to branch {to}, don't forget to push!");
    Ok(run_type)
}

pub(crate) fn select_issue_from_current_branch(run_type: RunType) -> Result<RunType, Error> {
    match run_type {
        RunType::DryRun {
            mut state,
            mut stdout,
        } => {
            writeln!(
                stdout,
                "Would attempt to parse current branch name to select current issue"
            )
            .map_err(fs::Error::Stdout)?;
            state.issue = state::Issue::Selected(Issue {
                key: String::from("123"),
                summary: String::from("Fake Issue"),
            });
            Ok(RunType::DryRun { state, stdout })
        }
        RunType::Real(mut state) => {
            let repo = Repository::open(".").map_err(ErrorKind::OpenRepo)?;
            let head = repo.head()?;
            let ref_name = head.name().ok_or(ErrorKind::NotOnAGitBranch)?;
            let issue = select_issue_from_branch_name(ref_name)?;
            state.issue = state::Issue::Selected(issue);
            Ok(RunType::Real(state))
        }
    }
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
        println!("Auto-selecting issue {github_issue} from ref {ref_name}");
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
    println!("Auto-selecting issue {jira_issue} from ref {ref_name}");
    return Ok(Issue {
        key: jira_issue,
        summary: parts.iter().join("-"),
    });
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
pub(crate) fn add_files(file_names: &[PathBuf]) -> Result<(), Error> {
    if file_names.is_empty() {
        return Ok(());
    }
    let repo = Repository::open(".").map_err(ErrorKind::OpenRepo)?;
    let mut index = repo.index()?;
    index.add_all(file_names, IndexAddOption::DEFAULT, None)?;
    index.write().map_err(Error::from)
}

/// Find every commit that appears only _after_ a specific tag.
///
/// This builds a complete set of every commit in the repository, because branching and merging
/// means that there could be paths which jump _behind_ the target tag... and we want to exclude
/// those as well. There's probably a way to optimize performance with some cool graph magic
/// eventually, but this is good enough for now.
pub(crate) fn get_commit_messages_after_tag(
    tag: Option<String>,
    verbose: Verbose,
) -> Result<Vec<String>, Error> {
    let repo = gix::open(".")?;
    let commits_to_exclude = tag
        .map(|tag| format!("refs/tags/{tag}"))
        .as_ref()
        .map(|reference| {
            if let Verbose::Yes = verbose {
                println!("Finding all commits behind {reference}");
            }
            repo.find_reference(reference)
                .map_err(|err| ErrorKind::FindReference {
                    reference: reference.clone(),
                    source: err,
                })
        })
        .transpose()?
        .map(gix::Reference::into_fully_peeled_id)
        .transpose()?
        .and_then(|tag_oid| repo.find_object(tag_oid).ok().map(gix::Object::into_commit))
        .and_then(|commit| {
            commit.ancestors().all().ok().map(|ancestors| {
                ancestors
                    .into_iter()
                    .filter_map(Result::ok)
                    .map(|info| info.id)
                    .collect::<HashSet<ObjectId>>()
            })
        })
        .unwrap_or_default();
    let head_commit = repo.head_commit()?;
    let mut reverse_commits = head_commit
        .ancestors()
        .all()?
        .filter_map(Result::ok)
        .filter(|info| !commits_to_exclude.contains(&info.id))
        .filter_map(|info| {
            info.object().ok().and_then(|commit| {
                commit
                    .decode()
                    .ok()
                    .map(|commit| commit.message.to_string())
            })
        })
        .collect_vec();
    reverse_commits.reverse();
    Ok(reverse_commits)
}

pub(crate) fn create_tag(dry_run: DryRun, name: String) -> Result<(), Error> {
    if let Some(stdout) = dry_run {
        return writeln!(stdout, "Would create Git tag {name}")
            .map_err(fs::Error::Stdout)
            .map_err(Error::from);
    }
    let repo = gix::open(current_dir().map_err(ErrorKind::CurrentDirectory)?)?;
    let head = repo.head_commit()?;
    repo.tag(
        name,
        head.id,
        Kind::Commit,
        repo.committer()
            .transpose()
            .map_err(|_| ErrorKind::NoCommitter)?,
        "",
        PreviousValue::Any,
    )?;
    Ok(())
}

pub(crate) fn get_current_versions_from_tags(
    prefix: Option<&str>,
    verbose: Verbose,
) -> Result<CurrentVersions, Error> {
    let repo = gix::open(current_dir().map_err(ErrorKind::CurrentDirectory)?)?;
    let pattern = prefix
        .as_ref()
        .map_or_else(|| String::from("v"), |prefix| format!("{prefix}/v"));
    let mut all_tags: HashMap<ObjectId, Vec<String>> = HashMap::new();
    for (id, tag) in repo
        .references()?
        .tags()?
        .filter_map(Result::ok)
        .filter_map(|mut reference| {
            reference.peel_to_id_in_place().ok().map(|id| {
                (
                    id.detach(),
                    reference
                        .name()
                        .as_bstr()
                        .to_string()
                        .replace("refs/tags/", ""),
                )
            })
        })
        .filter(|(_id, tag_name)| tag_name.starts_with(&pattern))
    {
        all_tags.entry(id).or_default().push(tag);
    }

    let mut tags: Vec<String> = Vec::with_capacity(all_tags.len());
    for commit_id in repo
        .head_commit()?
        .ancestors()
        .all()?
        .filter_map(|info| info.ok().map(|info| info.id))
    {
        if let Some(tag) = all_tags.remove(&commit_id) {
            tags.extend(tag);
        }
    }

    if let Verbose::Yes = verbose {
        if !all_tags.is_empty() {
            println!(
                "Skipping relevant tags that are not on the current branch: {tags}",
                tags = all_tags.values().flatten().join(", ")
            );
        }
        if tags.is_empty() {
            println!("No tags found matching pattern {pattern}");
        }
    }

    let mut current_versions = CurrentVersions::default();
    for tag in tags {
        let version_string = tag.replace(&pattern, "");
        if let Ok(version) = Version::from_str(version_string.as_str()) {
            current_versions.update_version(version);
        }
    }

    Ok(current_versions)
}
