use color_eyre::eyre::WrapErr;
use color_eyre::eyre::{eyre, Result};
use semver::{Identifier, Version};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "rule", content = "value")]
pub enum Rule {
    Major,
    Minor,
    Patch,
    Pre(String),
    Release,
}

pub(crate) fn bump_version(state: crate::State, rule: Rule) -> Result<crate::State> {
    if let Some(cargo_version) = crate::cargo::get_version() {
        let version = Version::parse(&cargo_version).wrap_err_with(|| {
            format!(
                "Found {} in Cargo.toml which is not a valid version",
                cargo_version
            )
        })?;
        let version = bump(version, &rule).wrap_err("While bumping Cargo.toml")?;
        crate::cargo::set_version(&version.to_string()).wrap_err("While bumping Cargo.toml")?;
    }
    Ok(state)
}

fn bump(mut version: Version, rule: &Rule) -> Result<Version> {
    match rule {
        Rule::Major => version.increment_major(),
        Rule::Minor => version.increment_minor(),
        Rule::Patch => version.increment_patch(),
        Rule::Release => {
            version.pre = Vec::new();
        }
        Rule::Pre(prefix) => {
            version = bump_pre(version, prefix)?;
        }
    };
    Ok(version)
}

fn bump_pre(mut version: Version, prefix: &str) -> Result<Version> {
    if version.pre.is_empty() {
        version.pre = vec![
            Identifier::AlphaNumeric(prefix.to_owned()),
            Identifier::Numeric(0),
        ];
        return Ok(version);
    } else if version.pre.len() != 2 {
        return Err(eyre!(
            "A prerelease version already exists but could not be incremented"
        ));
    }
    if let Some(Identifier::AlphaNumeric(existing_prefix)) = version.pre.get(0) {
        if existing_prefix != prefix {
            return Err(eyre!(
                "Found prefix {} which does not match provided prefix {}",
                existing_prefix,
                prefix
            ));
        }
    } else {
        return Err(eyre!(
            "A prerelease version already exists but could not be incremented"
        ));
    }
    match version.pre.remove(1) {
        Identifier::Numeric(pre_version) => {
            version.pre.insert(1, Identifier::Numeric(pre_version + 1));
            Ok(version)
        }
        _ => Err(eyre!("No numeric pre component to bump")),
    }
}
