use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use enum_iterator::{all, Sequence};
use itertools::Itertools;
use knope_versioning::cargo;
use miette::Diagnostic;

use crate::{
    fs,
    fs::read_to_string,
    step::releases::{
        versioned_file::{PackageFormat, VersionedFile},
        Package,
    },
};

#[derive(Clone, Copy, Debug, Sequence)]
enum WorkspaceFile {
    CargoToml,
}

pub(crate) fn check_for_workspaces() -> Result<Vec<Package>, Error> {
    all::<WorkspaceFile>()
        .map(WorkspaceFile::get_packages)
        .flatten_ok()
        .collect()
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
    #[error("Could not parse TOML in {1}: {0}")]
    #[diagnostic(code(workspace::toml))]
    Toml(toml::de::Error, PathBuf),
    #[error("Could not get parent directory of Cargo.toml file: {0}")]
    #[diagnostic(code(workspace::parent))]
    Parent(PathBuf),
    #[error("The Cargo workspace members array should contain only strings")]
    #[diagnostic(code(workspace::members))]
    Members,
}

impl WorkspaceFile {
    fn get_packages(self) -> Result<Vec<Package>, Error> {
        match self {
            Self::CargoToml => cargo_workspace_members(Path::new("Cargo.toml")),
        }
    }
}

fn cargo_workspace_members(path: &Path) -> Result<Vec<Package>, Error> {
    let Ok(contents) = read_to_string(path) else {
        return Ok(Vec::new());
    };
    let cargo_toml =
        toml::Value::from_str(&contents).map_err(|err| Error::Toml(err, path.into()))?;
    let workspace_path = path.parent().ok_or_else(|| Error::Parent(path.into()))?;
    let Some(members) = cargo_toml
        .get("workspace")
        .and_then(|workspace| workspace.as_table()?.get("members")?.as_array())
    else {
        return Ok(Vec::new());
    };
    members
        .iter()
        .map(|member_val| {
            let member = member_val.as_str().ok_or(Error::Members)?;
            let member_path = workspace_path.join(member).join("Cargo.toml");
            let member_contents = read_to_string(&member_path)?;
            toml::from_str::<cargo::Toml>(&member_contents)
                .map_err(|err| Error::Toml(err, member_path.clone()))
                .map(|cargo| {
                    Package::new(
                        cargo.package.name,
                        vec![VersionedFile {
                            format: PackageFormat::Cargo,
                            path: member_path,
                            content: member_contents,
                        }],
                    )
                })
        })
        .collect()
}
