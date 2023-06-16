use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    fmt::Display,
    fs,
    path::PathBuf,
};

use git_conventional::FooterToken;
use miette::{IntoDiagnostic, Result, WrapErr};
use serde::{Deserialize, Serialize};

use crate::{
    command, git, releases,
    releases::find_packages,
    step::{PrepareRelease, Step, StepError},
    workflow::Workflow,
};

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

    /// Create a Config from a TOML file or load the default config via `generate`
    ///
    /// ## Errors
    /// 1. Cannot parse file contents into a Config
    pub(crate) fn load() -> Result<Self> {
        let Ok(contents) = fs::read_to_string(Self::CONFIG_PATH) else {
            log::debug!("No `knope.toml` found, using default config");
            return Ok(generate());
        };
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

    /// Upgrade any deprecated syntax to the latest equivalent syntax.
    ///
    /// # Returns
    ///
    /// Whether or not any changes were made.
    #[must_use]
    pub(crate) fn upgrade(&mut self) -> bool {
        let mut upgraded = false;
        match self.packages.take() {
            Some(Packages::Multiple(packages)) => {
                self.packages = Some(Packages::Multiple(packages));
            }
            Some(Packages::Deprecated(packages)) => {
                println!("Upgrading deprecated [[packages]] syntax to [package]");
                upgraded = true;
                let [package] = packages;
                self.package = Some(package);
            }
            None => {}
        }
        upgraded
    }

    /// Write out the Config to `knope.toml`.
    pub(crate) fn write_out(&self) -> Result<()> {
        let contents = toml::to_string(&self).into_diagnostic()?;
        fs::write(Config::CONFIG_PATH, contents).into_diagnostic()
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
    /// Optional scopes that can be used to filter commits when running [`crate::Step::PrepareRelease`].
    pub(crate) scopes: Option<Vec<String>>,
    /// Extra sections that should be added to the changelog from custom footers in commit messages.
    pub(crate) extra_changelog_sections: Option<Vec<ChangelogSection>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ChangelogSection {
    pub(crate) name: ChangeLogSectionName,
    #[serde(default)]
    pub(crate) footers: Vec<CommitFooter>,
    #[serde(default)]
    pub(crate) types: Vec<CustomChangeType>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct CommitFooter(String);

impl Display for CommitFooter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<FooterToken<'_>> for CommitFooter {
    fn from(token: FooterToken<'_>) -> Self {
        Self(token.to_string())
    }
}

impl From<&'static str> for CommitFooter {
    fn from(token: &'static str) -> Self {
        Self(token.into())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct CustomChangeType(String);

impl Display for CustomChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for CustomChangeType {
    fn from(token: String) -> Self {
        Self(token)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct ChangeLogSectionName(String);

impl Display for ChangeLogSectionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&'static str> for ChangeLogSectionName {
    fn from(token: &'static str) -> Self {
        Self(token.into())
    }
}

impl AsRef<str> for ChangeLogSectionName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Generate a brand new Config for the project in the current directory.
pub(crate) fn generate() -> Config {
    let mut variables = HashMap::new();
    variables.insert(String::from("$version"), command::Variable::Version);

    let github = match git::get_first_remote() {
        Some(remote) if remote.contains("github.com") => {
            let parts = remote.split('/').collect::<Vec<_>>();
            let owner = parts.get(parts.len() - 2).map(|owner| {
                owner
                    .strip_prefix("git@github.com:")
                    .unwrap_or(owner)
                    .to_string()
            });

            let repo = parts
                .last()
                .map(|repo| repo.strip_suffix(".git").unwrap_or(repo).to_string());

            owner
                .and_then(|owner| repo.map(|repo| (owner, repo)))
                .map(|(owner, repo)| GitHub { owner, repo })
        }
        _ => None,
    };
    let mut release_steps = if github.is_some() {
        vec![
            Step::Command {
                command: String::from(
                    "git commit -m \"chore: prepare release $version\" && git push",
                ),
                variables: Some(variables),
            },
            Step::Release,
        ]
    } else {
        vec![
            Step::Command {
                command: String::from("git commit -m \"chore: prepare release $version\""),
                variables: Some(variables),
            },
            Step::Release,
            Step::Command {
                command: String::from("git push && git push --tags"),
                variables: None,
            },
        ]
    };
    release_steps.insert(
        0,
        Step::PrepareRelease(PrepareRelease {
            prerelease_label: None,
        }),
    );

    Config {
        workflows: vec![
            Workflow {
                name: String::from("release"),
                steps: release_steps,
            },
            Workflow {
                name: String::from("document-change"),
                steps: vec![Step::CreateChangeFile],
            },
        ],
        jira: None,
        github,
        package: find_packages(),
        packages: None,
    }
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
