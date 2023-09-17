#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::cargo)]
#![deny(warnings)]
#![allow(clippy::multiple_crate_versions)] // Let cargo-deny handle this

// Don't panic!
#![cfg_attr(
    not(test),
    deny(
        clippy::panic,
        clippy::exit,
        clippy::unimplemented,
        clippy::todo,
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::indexing_slicing,
        clippy::missing_panics_doc
    )
)]

use std::{io::stdout, str::FromStr};

use clap::{arg, command, value_parser, Arg, ArgAction, ArgMatches, Command};
use itertools::Itertools;
use miette::{miette, Result};

use crate::{
    config::{Config, ConfigSource},
    state::{RunType, State},
    step::{
        releases::{semver::Version, PackageName},
        Step,
    },
    workflow::{Verbose, Workflow},
};

mod app_config;
mod config;
mod dry_run;
mod fs;
mod integrations;
mod prompt;
mod state;
mod step;
mod variables;
mod workflow;

/// The main entry point for the application.
///
/// # Errors
///
/// 1. `knope.toml` not found
/// 2. `knope.toml` not valid
/// 3. Selected workflow not found
/// 4. Passthrough errors of selected workflow
pub fn run() -> Result<()> {
    let config = Config::load()?;

    let mut matches = build_cli(&config).get_matches();

    let mut config = config.into_inner();
    let verbose = matches.get_flag(VERBOSE).into();

    if let Ok(Some(true)) = matches.try_get_one("generate") {
        println!("Generating a knope.toml file");
        let config = config::generate();
        return config.write_out();
    }

    if let Ok(Some(true)) = matches.try_get_one("upgrade") {
        // If adding new upgrade, make a function to detect and call here.
        let upgraded = false;
        return if upgraded {
            config.write_out()
        } else {
            println!("Nothing to upgrade");
            Ok(())
        };
    }

    let (subcommand, mut sub_matches) = matches.remove_subcommand().unzip();

    sub_matches.as_ref().and_then(|matches| {
        matches
            .try_get_one::<String>("prerelease-label")
            .ok()
            .flatten()
            .map(|prerelease_label| {
                config.set_prerelease_label(prerelease_label);
            })
    });

    let (state, workflows) = create_state(config, sub_matches.as_mut(), verbose)?;

    if let Ok(Some(true)) = matches.try_get_one("validate") {
        workflow::validate(workflows, state)?;
        return Ok(());
    }

    let subcommand = subcommand.ok_or_else(|| {
        miette!("No workflow selected. Run `knope --help` for a list of options.")
    })?;
    let workflow = workflows
        .into_iter()
        .find(|w| w.name == subcommand)
        .ok_or_else(|| miette!("No workflow named {}", subcommand))?;

    let state = if matches.get_flag("dry-run") {
        RunType::DryRun {
            state,
            stdout: Box::new(stdout()),
        }
    } else {
        RunType::Real(state)
    };

    workflow::run(workflow, state)?;
    Ok(())
}

const OVERRIDE_ONE_VERSION: &str = "override-one-version";
const OVERRIDE_MULTIPLE_VERSIONS: &str = "override-multiple-versions";
const PRERELEASE_LABEL: &str = "prerelease-label";
const VERBOSE: &str = "verbose";

fn build_cli(config: &ConfigSource) -> Command {
    let mut command = command!()
        .propagate_version(true)
        .arg(
            Arg::new("dry-run").long("dry-run")
                .help("Pretend to run a workflow, outputting what _would_ happen without actually doing it.")
                .action(ArgAction::SetTrue)
                .global(true)
        ).arg(
        Arg::new(VERBOSE).long(VERBOSE).short('v')
            .help("Print extra information (for debugging)")
            .action(ArgAction::SetTrue)
            .global(true)
    );
    let config = match config {
        ConfigSource::Default(config) => {
            command = command
                .arg(arg!(--generate "Generate a knope.toml file").action(ArgAction::SetTrue));
            config
        }
        ConfigSource::File(config) => {
            command = command.arg(arg!(--upgrade "Upgrade to the latest `knope.toml` syntax from any deprecated (but still supported) syntax."));
            command = command.arg(arg!(--validate "Check that the `knope.toml` file is valid."));
            config
        }
    };

    let version_override_arg = if config.packages.is_empty() {
        None
    } else if config.packages.len() == 1 {
        Some(Arg::new(OVERRIDE_ONE_VERSION)
            .long("override-version")
            .help("Override the version set by `BumpVersion` or `PrepareRelease` for the package.")
            .value_parser(value_parser!(Version)))
    } else {
        Some(Arg::new(OVERRIDE_MULTIPLE_VERSIONS)
            .long("override-version")
            .help("Override the version set by `BumpVersion` or `PrepareRelease` for multiple packages. Format is like package_name=version, can be set multiple times.")
            .action(ArgAction::Append).value_parser(value_parser!(VersionOverride)))
    };

    for workflow in &config.workflows {
        let mut subcommand = Command::new(workflow.name.clone());
        let contains_bump_version = workflow
            .steps
            .iter()
            .any(|step| matches!(*step, Step::BumpVersion(_)));
        let contains_prepare_release = workflow
            .steps
            .iter()
            .any(|step| matches!(*step, Step::PrepareRelease(_)));
        if contains_bump_version || contains_prepare_release {
            if let Some(arg) = version_override_arg.clone() {
                subcommand = subcommand.arg(arg);
            }
        }
        if contains_prepare_release {
            subcommand = subcommand
                .arg(
                    Arg::new(PRERELEASE_LABEL)
                        .long("prerelease-label")
                        .help("Set the `prerelease_label` attribute of any `PrepareRelease` steps at runtime.")
                        .env("KNOPE_PRERELEASE_LABEL")
                );
        }

        command = command.subcommand(subcommand);
    }
    command
}

fn create_state(
    config: Config,
    mut sub_matches: Option<&mut ArgMatches>,
    verbose: Verbose,
) -> Result<(State, Vec<Workflow>)> {
    let Config {
        mut packages,
        workflows,
        jira,
        github,
    } = config;
    if let Some(version_override) = sub_matches
        .as_deref_mut()
        .and_then(|matches| matches.try_remove_one::<Version>(OVERRIDE_ONE_VERSION).ok())
        .flatten()
    {
        if let Some(package) = packages.first_mut() {
            package.override_version = Some(version_override);
        }
    } else {
        let mut overrides = sub_matches
            .and_then(|matches| {
                matches
                    .try_remove_many::<VersionOverride>(OVERRIDE_MULTIPLE_VERSIONS)
                    .ok()
            })
            .into_iter()
            .flatten()
            .flatten()
            .collect_vec();
        for package in &mut packages {
            let override_index = overrides
                .iter()
                .find_position(|version_override| {
                    package
                        .name
                        .as_ref()
                        .is_some_and(|name| *name == version_override.package)
                })
                .map(|(index, _)| index);

            let version = override_index
                .map(|index| overrides.remove(index))
                .map(|version_override| version_override.version);

            package.override_version = version;
        }
        if !overrides.is_empty() {
            return Err(miette!(
                "Unknown package(s) to override: {}",
                overrides
                    .into_iter()
                    .map(|version_override| version_override.package.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    let state = State::new(jira, github, packages, verbose);
    Ok((state, workflows))
}

#[derive(Clone, Debug)]
struct VersionOverride {
    package: PackageName,
    version: Version,
}

impl FromStr for VersionOverride {
    type Err = miette::Report;

    fn from_str(s: &str) -> Result<Self> {
        let (package, version_string) = s.split_once('=').ok_or_else(|| {
            miette!("package override should be formatted like package_name=version")
        })?;

        Ok(Self {
            package: package.into(),
            version: version_string.parse()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_app() {
        build_cli(&ConfigSource::Default(config::generate())).debug_assert();
    }
}
