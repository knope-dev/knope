use git_repository::reference::head_commit;
use git_repository::tag;
use std::collections::HashMap;
use std::path::PathBuf;

use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::releases::suggested_package_toml;
use crate::state::RunType;
use crate::{command, git, issues, releases};

/// Each variant describes an action you can take using knope, they are used when defining your
/// [`crate::Workflow`] via whatever config format is being utilized.
#[derive(Deserialize, Debug, Serialize)]
#[serde(tag = "type")]
pub(crate) enum Step {
    /// Search for Jira issues by status and display the list of them in the terminal.
    /// User is allowed to select one issue which will then change the workflow's state to
    /// [`State::IssueSelected`].
    SelectJiraIssue {
        /// Issues with this status in Jira will be listed for the user to select.
        status: String,
    },
    /// Transition a Jira issue to a new status.
    TransitionJiraIssue {
        /// The status to transition the current issue to.
        status: String,
    },
    /// Search for GitHub issues by status and display the list of them in the terminal.
    /// User is allowed to select one issue which will then change the workflow's state to
    /// [`State::IssueSelected`].
    SelectGitHubIssue {
        /// If provided, only issues with this label will be included
        labels: Option<Vec<String>>,
    },
    /// Attempt to parse issue info from the current branch name and change the workflow's state to
    /// [`State::IssueSelected`].
    SelectIssueFromBranch,
    /// Uses the name of the currently selected issue to checkout an existing or create a new
    /// branch for development. If an existing branch is not found, the user will be prompted to
    /// select an existing local branch to base the new branch off of. Remote branches are not
    /// shown.
    SwitchBranches,
    /// Rebase the current branch onto the branch defined by `to`.
    RebaseBranch {
        /// The branch to rebase onto.
        to: String,
    },
    /// Bump the version of the project in any supported formats found using a
    /// [Semantic Versioning](https://semver.org) rule.
    BumpVersion(releases::Rule),
    /// Run a command in your current shell after optionally replacing some variables.
    Command {
        /// The command to run, with any variable keys you wish to replace.
        command: String,
        /// A map of value-to-replace to [Variable][`crate::command::Variable`] to replace
        /// it with.
        variables: Option<HashMap<String, command::Variable>>,
    },
    /// This will look through all commits since the last tag and parse any
    /// [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) it finds. It will
    /// then bump the project version (depending on the rule determined from the commits) and add
    /// a new Changelog entry using the [Keep A Changelog](https://keepachangelog.com/en/1.0.0/)
    /// format.
    PrepareRelease(PrepareRelease),
    /// This will create a new release on GitHub using the current project version.
    ///
    /// Requires that GitHub details be configured.
    Release,
}

impl Step {
    pub(crate) fn run(self, run_type: RunType) -> Result<RunType, StepError> {
        match self {
            Step::SelectJiraIssue { status } => issues::select_jira_issue(&status, run_type),
            Step::TransitionJiraIssue { status } => {
                issues::transition_jira_issue(&status, run_type)
            }
            Step::SelectGitHubIssue { labels } => {
                issues::select_github_issue(labels.as_deref(), run_type)
            }
            Step::SwitchBranches => git::switch_branches(run_type),
            Step::RebaseBranch { to } => git::rebase_branch(&to, run_type),
            Step::BumpVersion(rule) => releases::bump_version(run_type, rule),
            Step::Command { command, variables } => {
                command::run_command(run_type, command, variables)
            }
            Step::PrepareRelease(prepare_release) => {
                releases::prepare_release(run_type, prepare_release)
            }
            Step::SelectIssueFromBranch => git::select_issue_from_current_branch(run_type),
            Step::Release => releases::release(run_type),
        }
    }
}

#[derive(Debug, Error, Diagnostic)]
pub(super) enum StepError {
    #[error("No issue selected")]
    #[diagnostic(
        code(step::no_issue_selected),
        help("You must call SelectJiraIssue or SelectGitHubIssue before calling this step")
    )]
    NoIssueSelected,
    #[error("Jira is not configured")]
    #[diagnostic(
        code(step::jira_not_configured),
        help("Jira must be configured in order to call this step"),
        url("https://knope-dev.github.io/knope/config/jira.html")
    )]
    JiraNotConfigured,
    #[error("The specified transition name was not found in the Jira project")]
    #[diagnostic(
    code(step::invalid_jira_transition),
    help("The `transition` field in TransitionJiraIssue must correspond to a valid transition in the Jira project"),
    url("https://knope-dev.github.io/knope/config/jira.html")
    )]
    InvalidJiraTransition,
    #[error("GitHub is not configured")]
    #[diagnostic(
        code(step::github_not_configured),
        help("GitHub must be configured in order to call this step"),
        url("https://knope-dev.github.io/knope/config/github.html")
    )]
    GitHubNotConfigured,
    #[error("Could not increment pre-release version {0}")]
    #[diagnostic(
        code(step::invalid_pre_release_version),
        help(
            "The pre-release component of a version must be in the format of `-<label>.N` \
            where <label> is a string and `N` is an integer"
        ),
        url("https://knope-dev.github.io/knope/config/step/BumpVersion.html#pre")
    )]
    InvalidPreReleaseVersion(String),
    #[error("Found invalid semantic version {0}")]
    #[diagnostic(
        code(step::invalid_semantic_version),
        help("The version must be a valid Semantic Version"),
        url("https://knope-dev.github.io/knope/config/packages.html#versioned_files")
    )]
    InvalidSemanticVersion(String),
    #[error("Versioned files within the same package must have the same version. Found {0} which does not match {1}")]
    #[diagnostic(
        code(step::inconsistent_versions),
        help("Manually update all versioned_files to have the correct version"),
        url("https://knope-dev.github.io/knope/config/step/BumpVersion.html")
    )]
    InconsistentVersions(String, String),
    #[error("The versioned file {0} is not a supported format")]
    #[diagnostic(
        code(step::versioned_file_format),
        help("All filed included in [[packages]] versioned_files must be a supported format"),
        url("https://knope-dev.github.io/knope/config/packages.html#supported-formats-for-versioning")
    )]
    VersionedFileFormat(PathBuf),
    #[error("The package.json file was an incorrect format")]
    #[diagnostic(
        code(step::invalid_package_json),
        help("knope expects the package.json file to be an object with a top level `version` property"),
        url("https://knope-dev.github.io/knope/config/packages.html#supported-formats-for-versioning")
    )]
    InvalidPackageJson,
    #[error("The pyproject.toml file was an incorrect format")]
    #[diagnostic(
        code(step::invalid_pyproject),
        help(
            "knope expects the pyproject.toml file to have a `tool.poetry.version` property. \
            If you use a different location for your version, please open an issue to add support."
        ),
        url("https://knope-dev.github.io/knope/config/packages.html#supported-formats-for-versioning")
    )]
    InvalidPyProject,
    #[error("The Cargo.toml file was an incorrect format")]
    #[diagnostic(
        code(step::invalid_cargo_toml),
        help("knope expects the Cargo.toml file to have a `package.version` property. Workspace support is coming soon!"),
        url("https://knope-dev.github.io/knope/config/packages.html#supported-formats-for-versioning")
    )]
    InvalidCargoToml,
    #[error("Trouble communicating with a remote API")]
    #[diagnostic(
        code(step::api_request_error),
        help(
            "This occurred during a step that requires communicating with a remote API \
             (e.g., GitHub or Jira). The problem could be an invalid authentication token or a \
             network issue."
        )
    )]
    ApiRequestError(#[from] ureq::Error),
    #[error("Trouble decoding the response from a remote API")]
    #[diagnostic(
    code(step::api_response_error),
    help(
    "This occurred during a step that requires communicating with a remote API \
             (e.g., GitHub or Jira). If we were unable to decode the response, it's probably a bug."
    )
    )]
    ApiResponseError(#[source] Option<serde_json::Error>),
    #[error("I/O error")]
    #[diagnostic(
        code(step::io_error),
        help(
            "This occurred during a step that requires reading or writing to... something. The \
            problem could be a network issue or a file permission issue."
        )
    )]
    IoError(#[from] std::io::Error),
    #[error("Not a Git repo.")]
    #[diagnostic(
    code(step::not_a_git_repo),
    help(
    "We couldn't find a Git repo in the current directory. Maybe you're not running from the project root?"
    )
    )]
    NotAGitRepo,
    #[error("Not on the tip of a Git branch.")]
    #[diagnostic(
        code(step::not_on_a_git_branch),
        help("In order to run this step, you need to be on the very tip of a Git branch.")
    )]
    NotOnAGitBranch,
    #[error("Bad branch name")]
    #[diagnostic(
        code(step::bad_branch_name),
        help("The branch name was not formatted correctly."),
        url("https://knope-dev.github.io/knope/config/step/SelectIssueFromBranch.html")
    )]
    BadGitBranchName,
    #[error("Uncommitted changes")]
    #[diagnostic(
        code(step::uncommitted_changes),
        help("You need to commit your changes before running this step."),
        url("https://knope-dev.github.io/knope/config/step/SwitchBranches.html")
    )]
    UncommittedChanges,
    #[error("Could not complete checkout")]
    #[diagnostic(
    code(step::incomplete_checkout),
    help("Switching branches failed, but HEAD was changed. You probably want to git switch back \
        to the branch you were on."),
    )]
    IncompleteCheckout(#[source] git2::Error),
    #[error("Could not list tags for the project")]
    #[diagnostic(
    code(step::list_tags_error),
    help("We couldn't list the tags for the project. This step requires at least one existing tag."),
    url("https://knope-dev.github.io/knope/config/step/PrepareRelease.html")
    )]
    ListTagsError(#[source] git2::Error),
    #[error("Unknown Git error.")]
    #[diagnostic(
    code(step::git_error),
    help(
    "Something went wrong when interacting with Git that we don't have an explanation for. \
            Maybe try performing the operation manually?"
    )
    )]
    GitError(#[from] Option<git2::Error>),
    #[error("Could not get head commit")]
    #[diagnostic(
        code(step::head_commit_error),
        help("This step requires HEAD to point to a commit—it was not.")
    )]
    HeadCommitError(#[from] head_commit::Error),
    #[error("Could not create a tag")]
    #[diagnostic(
        code(step::create_tag_error),
        help("A Git tag could not be created for the release.")
    )]
    CreateTagError(#[from] tag::Error),
    #[error("Command returned non-zero exit code")]
    #[diagnostic(
        code(step::command_failed),
        help("The command failed to execute. Try running it manually to get more information.")
    )]
    CommandError(std::process::ExitStatus),
    #[error("Failed to get user input")]
    #[diagnostic(
    code(step::user_input_error),
    help("This step requires user input, but no user input was provided. Try running the step again."),
    )]
    UserInput(#[source] Option<std::io::Error>),
    #[error("This is a bug!")]
    #[diagnostic(
        code(step::bug),
        help("If you see this error, it's a bug in knope! Please report it in GitHub."),
        url("https://github.com/knope-dev/knope/issues")
    )]
    Bug(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("PrepareRelease needs to occur before this step")]
    #[diagnostic(
        code(step::release_not_prepared),
        help("You must call the PrepareRelease step before this one."),
        url("https://knope-dev.github.io/knope/config/step/PrepareRelease.html")
    )]
    ReleaseNotPrepared,
    #[error("No packages are defined")]
    #[diagnostic(
        code(step::no_defined_packages),
        help("You must define at least one package in the [[packages]] section of knope.toml. {package_suggestion}"),
        url("https://knope-dev.github.io/knope/config/packages.html")
    )]
    NoDefinedPackages { package_suggestion: String },
    #[error("Too many packages defined")]
    #[diagnostic(
        code(step::too_many_packages),
        help("Only one package in [[packages]] is currently supported."),
        url("https://github.com/knope-dev/knope/issues/153")
    )]
    TooManyPackages,
    #[error("No versioned files defined")]
    #[diagnostic(
    code(step::no_versioned_files),
    help("You must define at least one versioned_files per package in the [[packages]] section of knope.toml."),
    url("https://knope-dev.github.io/knope/config/packages.html")
    )]
    NoVersionedFiles,
    #[error("File {0} does not exist")]
    #[diagnostic(
        code(step::file_not_found),
        help("Attempted to interact with a file that doesn't exist in the current directory.")
    )]
    FileNotFound(PathBuf),
}

impl StepError {
    pub fn no_defined_packages_with_help() -> Self {
        Self::NoDefinedPackages {
            package_suggestion: suggested_package_toml(),
        }
    }
}

/// The inner content of a [`Step::PrepareRelease`] step.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct PrepareRelease {
    /// If set, the user wants to create a pre-release version using the selected label.
    pub(crate) prerelease_label: Option<String>,
}
