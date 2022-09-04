use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use miette::{IntoDiagnostic, Result, WrapErr};
use serde::{Deserialize, Serialize};
use velcro::{hash_map, vec};

use crate::releases::find_packages;
use crate::step::{PrepareRelease, Step, StepError};
use crate::workflow::Workflow;
use crate::{command, git, releases};

#[derive(Deserialize, Debug, Serialize)]
pub(crate) struct Config {
    /// A list of defined packages within this project which can be updated via PrepareRelease or BumpVersion
    #[serde(default, skip_serializing_if = "Option::is_none")]
    packages: Option<Packages>,
    /// A single package to update via PrepareRelease or BumpVersion. Mutually exclusive with `packages`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    package: Option<Package>,
    /// The list of defined workflows that are selectable
    pub(crate) workflows: Vec<Workflow>,
    /// Optional configuration for Jira
    pub(crate) jira: Option<Jira>,
    /// Optional configuration to talk to GitHub
    pub(crate) github: Option<GitHub>,
}

impl Config {
    const CONFIG_PATH: &'static str = "knope.toml";

    /// Create a Config from a TOML file.
    ///
    /// ## Errors
    /// 1. Provided path is not found
    /// 2. Cannot parse file contents into a Config
    pub(crate) fn load() -> Result<Self> {
        let contents = fs::read_to_string(Self::CONFIG_PATH)
            .into_diagnostic()
            .wrap_err_with(|| {
                format!(
                    "Could not find {CONFIG_PATH}",
                    CONFIG_PATH = Self::CONFIG_PATH
                )
            })?;
        toml::from_str(&contents)
            .into_diagnostic()
            .wrap_err("Invalid TOML when parsing config")
    }

    /// Set the prerelease label for all `PrepareRelease` steps in all workflows in `self`.
    pub(crate) fn set_prerelease_label(&mut self, label: &str) {
        for workflow in &mut self.workflows {
            workflow.set_prerelease_label(label);
        }
    }

    pub(crate) fn packages(&self) -> Result<Vec<releases::Package>, StepError> {
        match (self.packages.clone(), self.package.clone()) {
            (None, None) => Ok(Vec::new()),
            (Some(..), Some(..)) => Err(StepError::ConflictingPackages),
            (None, Some(package)) => Ok(vec![releases::Package::new(package, None)?]),
            (Some(Packages::Multiple(packages)), None) => packages
                .into_iter()
                .map(|(name, package)| releases::Package::new(package, Some(name)))
                .collect(),
            (Some(Packages::Deprecated(packages)), None) => {
                println!("WARNING: The [[packages]] syntax is deprecated, use [package] instead. Run knope --upgrade to do this automatically.");
                packages
                    .into_iter()
                    .map(|package| releases::Package::new(package, None))
                    .collect()
            }
        }
    }
}

/// All of the different ways packages can be defined in `knope.toml`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub(crate) enum Packages {
    Multiple(BTreeMap<String, Package>),
    Deprecated([Package; 1]),
}

/// Represents a single package in `knope.toml`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Package {
    /// The files which define the current version of the package.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) versioned_files: Vec<PathBuf>,
    /// The path to the `CHANGELOG.md` file (if any) to be updated when running [`crate::Step::PrepareRelease`].
    pub(crate) changelog: Option<PathBuf>,
}

/// Generate a brand new config file for the project in the current directory.
pub(crate) fn generate() -> Result<()> {
    let variables = hash_map! {
        String::from("$version"): command::Variable::Version,
    };
    let mut github = None;

    let release_steps = match git::get_first_remote() {
        Some(remote) if remote.contains("github.com") => {
            let parts = remote.split('/').collect::<Vec<_>>();
            let owner = parts[parts.len() - 2].to_string();
            let repo = parts[parts.len() - 1].to_string();
            github = Some(GitHub { owner, repo });
            vec![
                Step::Command {
                    command: String::from("git add . && git commit -m \"chore: prepare release $version\" && git push"),
                    variables: Some(variables),
                },
                Step::Release,
            ]
        }
        _ => vec![
            Step::Command {
                command: String::from(
                    "git add . && git commit -m \"chore: prepare release $version\"",
                ),
                variables: Some(variables),
            },
            Step::Release,
            Step::Command {
                command: String::from("git push && git push --tags"),
                variables: None,
            },
        ],
    };

    let contents = toml::to_string(&Config {
        workflows: vec![Workflow {
            name: String::from("release"),
            steps: vec![
                Step::PrepareRelease(PrepareRelease {
                    prerelease_label: None,
                }),
                ..release_steps,
            ],
        }],
        jira: None,
        github,
        package: find_packages(),
        packages: None,
    })
    .unwrap();
    fs::write(Config::CONFIG_PATH, contents).into_diagnostic()
}

/// Config required for steps that interact with Jira.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct Jira {
    /// The URL to your Atlassian instance running Jira
    pub(crate) url: String,
    /// The key of the Jira project to filter on (the label of all issues)
    pub(crate) project: String,
}

/// Details needed to use steps that interact with GitHub.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct GitHub {
    /// The user or organization that owns the `repo`.
    pub(crate) owner: String,
    /// The name of the repository in GitHub that this project is utilizing
    pub(crate) repo: String,
}
