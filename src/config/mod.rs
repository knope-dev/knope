use std::fs;

use ::toml::{from_str, to_string, Spanned};
use indexmap::IndexMap;
use miette::{Diagnostic, IntoDiagnostic, Result, SourceSpan};
use serde::Serialize;
use thiserror::Error;

use crate::{
    config::toml::ConfigLoader,
    integrations::git,
    step::{
        releases::{find_packages, Package},
        PrepareRelease, Step,
    },
    variables::Variable,
    workflow::Workflow,
};

pub(crate) mod toml;

pub(crate) use self::toml::{
    ChangeLogSectionName, CommitFooter, CustomChangeType, GitHub, Gitea, Jira,
};

/// A valid config, loaded from a supported file (or detected via default)
#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) packages: Vec<Package>,
    /// The list of defined workflows that are selectable
    pub(crate) workflows: Vec<Workflow>,
    /// Optional configuration for Jira
    pub(crate) jira: Option<Jira>,
    /// Optional configuration to talk to GitHub
    pub(crate) github: Option<GitHub>,
    /// Optional configuration to communicate with a Gitea instance
    pub(crate) gitea: Option<Gitea>,
}

impl Config {
    const CONFIG_PATH: &'static str = "knope.toml";

    /// Create a Config from a TOML file or load the default config via `generate`
    ///
    /// ## Errors
    /// 1. Cannot parse file contents into a Config
    pub(crate) fn load() -> Result<ConfigSource, Error> {
        let Ok(source_code) = fs::read_to_string(Self::CONFIG_PATH) else {
            log::debug!("No `knope.toml` found, using default config");
            return Ok(ConfigSource::Default(generate()));
        };

        let config_loader: ConfigLoader = from_str(&source_code)?;
        let config_source = Self::try_from((config_loader, source_code)).map(ConfigSource::File)?;
        Ok(config_source.fill_in_gaps())
    }

    /// Set the prerelease label for all `PrepareRelease` steps in all workflows in `self`.
    pub(crate) fn set_prerelease_label(&mut self, label: &str) {
        for workflow in &mut self.workflows {
            workflow.set_prerelease_label(label);
        }
    }

    /// Write out the Config to `knope.toml`.
    pub(crate) fn write_out(mut self) -> Result<()> {
        #[derive(Serialize)]
        struct SimpleConfig {
            #[serde(skip_serializing_if = "Option::is_none")]
            package: Option<toml::Package>,
            workflows: Vec<Workflow>,
            github: Option<GitHub>,
            gitea: Option<Gitea>,
        }

        let config = SimpleConfig {
            package: self.packages.pop().map(toml::Package::from),
            workflows: self.workflows,
            github: self.github,
            gitea: self.gitea,
        };
        #[allow(clippy::unwrap_used)] // because serde is annoying... I know it will serialize
        let serialized = to_string(&config).unwrap();

        fs::write(Config::CONFIG_PATH, serialized).into_diagnostic()
    }
}

impl TryFrom<(ConfigLoader, String)> for Config {
    type Error = Error;

    fn try_from(
        (config, source_code): (ConfigLoader, String),
    ) -> std::result::Result<Self, Self::Error> {
        let packages = match (config.package, config.packages) {
            (Some(package), Some(packages)) => {
                return if let Some(first_packages) = packages.first() {
                    Err(Error::ConflictingPackages {
                        source_code,
                        package_definition: package.span().into(),
                        packages_definition: first_packages.1.span().into(),
                    })
                } else {
                    Err(Error::EmptyPackages)
                }
            }
            (Some(package), None) => {
                let span = package.span();
                vec![package
                    .into_inner()
                    .try_into()
                    .map_err(|err| Error::PackageFormat {
                        inner: err,
                        source_code,
                        span: span.into(),
                    })?]
            }
            (None, Some(packages)) => packages
                .into_iter()
                .map(|(name, config)| {
                    let span = config.span();
                    Package::try_from((name, config.into_inner())).map_err(|err| {
                        Error::PackageFormat {
                            inner: err,
                            source_code: source_code.clone(),
                            span: span.into(),
                        }
                    })
                })
                .collect::<Result<Vec<Package>, Error>>()?,
            (None, None) => Vec::new(),
        };

        if config.gitea.is_some()
            && packages.iter().any(|package| {
                package
                    .assets
                    .as_ref()
                    .is_some_and(|assets| !assets.is_empty())
            })
        {
            return Err(Error::GiteaAssetUploads);
        }

        Ok(Self {
            packages,
            workflows: config
                .workflows
                .into_inner()
                .into_iter()
                .map(Spanned::into_inner)
                .collect(),
            jira: config.jira.map(Spanned::into_inner),
            github: config.github.map(Spanned::into_inner),
            gitea: config.gitea.map(Spanned::into_inner),
        })
    }
}

/// Where the config came from
pub(crate) enum ConfigSource {
    /// There is no config file, this is the default config.
    Default(Config),
    /// Config loaded from a file.
    File(Config),
    /// Some things were loaded from file, some were defaults
    Hybrid(Config),
}

impl ConfigSource {
    pub(crate) fn into_inner(self) -> Config {
        match self {
            Self::File(config) | Self::Default(config) | Self::Hybrid(config) => config,
        }
    }

    /// Anything the config file was missing, fill in with defaults.
    fn fill_in_gaps(self) -> Self {
        let mut config = match self {
            Self::Hybrid(_) | Self::Default(_) => return self,
            Self::File(config) => config,
        };
        if config.packages.is_empty() {
            config.packages = find_packages().ok().unwrap_or_default();
        }
        if config.workflows.is_empty() {
            config.workflows = generate_workflows(
                config.github.is_some() || config.gitea.is_some(),
                &config.packages,
            );
        }
        Self::Hybrid(config)
    }
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(
        code(config::toml),
        help("Check the TOML is valid."),
        url("https://knope.tech/reference/config-file/github/")
    )]
    Toml(#[from] ::toml::de::Error),
    #[error("You cannot define both `packages` and `package`")]
    #[diagnostic(
        code(config::conflicting_packages),
        help("Add the `package` as a key under `packages` instead."),
        url("https://knope.tech/reference/config-file/packages/")
    )]
    ConflictingPackages {
        #[source_code]
        source_code: String,
        #[label("`package` defined here")]
        package_definition: SourceSpan,
        #[label("`packages` defined here")]
        packages_definition: SourceSpan,
    },
    #[error("The package definition is invalid: {inner}")]
    #[diagnostic(
        code(config::package_format),
        help("Check the package definition is valid."),
        url("https://knope.tech/reference/config-file/packages/")
    )]
    PackageFormat {
        inner: toml::package::Error,
        #[label("defined here")]
        span: SourceSpan,
        #[source_code]
        source_code: String,
    },
    #[error("The `packages` key is defined but does not contain any packages")]
    #[diagnostic(
        code(config::empty_packages),
        help("Add at least one package to the `packages` key."),
        url("https://knope.tech/reference/config-file/packages/")
    )]
    EmptyPackages,
    #[error("Asset uploads for Gitea are not supported")]
    #[diagnostic(
        code(config::gitea_asset_uploads),
        help("Remove the `[[package.assets]]` key from your config."),
        url("https://github.com/knope-dev/knope/issues/779")
    )]
    GiteaAssetUploads,
}

#[cfg(test)]
mod test_errors {

    use super::Config;

    #[test]
    fn conflicting_format() {
        let toml_string = r#"
            package = {}
            [packages.something]
            [[workflows]]
            name = "default"
            [[workflows.steps]]
            type = "Command"
            command = "echo this is nothing, really"
        "#
        .to_string();
        let config: super::toml::ConfigLoader = toml::from_str(&toml_string).unwrap();
        let config = Config::try_from((config, toml_string));
        assert!(config.is_err(), "Expected an error, got {config:?}");
    }

    #[test]
    fn gitea_asset_error() {
        let toml_string = r#"
            [packages.something]
            [[packages.something.assets]]
            name = "something"
            path = "something"
            [[workflows]]
            name = "default"
            [[workflows.steps]]
            type = "Command"
            command = "echo this is nothing, really"
            [gitea]
            host = "https://gitea.example.com"
            owner = "knope"
            repo = "knope"
        "#
        .to_string();
        let config: super::toml::ConfigLoader = toml::from_str(&toml_string).unwrap();
        let config = Config::try_from((config, toml_string));
        assert!(config.is_err(), "Expected an error, got {config:?}");
    }
}

/// Generate a brand new Config for the project in the current directory.
pub(crate) fn generate() -> Config {
    let packages = find_packages().ok().unwrap_or_default();

    let first_remote = git::get_first_remote();
    let github = match first_remote {
        Some(ref remote) if remote.contains("github.com") => {
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

    let gitea = first_remote.as_ref().and_then(|remote| {
        if Gitea::KNOWN_PUBLIC_GITEA_HOSTS
            .iter()
            .any(|known_host| remote.contains(known_host))
        {
            Gitea::try_from_remote(remote)
        } else {
            None
        }
    });

    Config {
        workflows: generate_workflows(github.is_some() || gitea.is_some(), &packages),
        jira: None,
        github,
        gitea,
        packages,
    }
}

fn generate_workflows(has_forge: bool, packages: &[Package]) -> Vec<Workflow> {
    let (commit_message, variables) = if packages.len() < 2 {
        let mut variables = IndexMap::new();
        variables.insert(String::from("$version"), Variable::Version);
        ("chore: prepare release $version", Some(variables))
    } else {
        ("chore: prepare releases", None)
    };

    let mut release_steps = if has_forge {
        vec![
            Step::Command {
                command: format!("git commit -m \"{commit_message}\" && git push",),
                variables,
            },
            Step::Release,
        ]
    } else {
        vec![
            Step::Command {
                command: format!("git commit -m \"{commit_message}\""),
                variables,
            },
            Step::Release,
            Step::Command {
                command: String::from("git push && git push --tags"),
                variables: None,
            },
        ]
    };
    release_steps.insert(0, Step::PrepareRelease(PrepareRelease::default()));
    vec![
        Workflow {
            name: String::from("release"),
            steps: release_steps,
        },
        Workflow {
            name: String::from("document-change"),
            steps: vec![Step::CreateChangeFile],
        },
    ]
}
