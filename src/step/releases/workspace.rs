use std::{path::Path, str::FromStr};

use enum_iterator::{all, Sequence};

use crate::{
    fs::read_to_string,
    step::releases::{
        cargo::CargoPackage,
        versioned_file::{PackageFormat, VersionedFile},
        Package,
    },
};

#[derive(Clone, Copy, Debug, Sequence)]
enum WorkspaceFile {
    CargoToml,
}

pub(crate) fn check_for_workspaces() -> Vec<Package> {
    all::<WorkspaceFile>()
        .filter_map(WorkspaceFile::get_packages)
        .flatten()
        .collect()
}

impl WorkspaceFile {
    fn get_packages(self) -> Option<Vec<Package>> {
        match self {
            Self::CargoToml => cargo_workspace_members(Path::new("Cargo.toml")),
        }
    }
}

fn cargo_workspace_members(path: &Path) -> Option<Vec<Package>> {
    let contents = read_to_string(path).ok()?;
    let cargo_toml = toml::Value::from_str(&contents).ok()?;
    let workspace_path = path.parent()?;
    Some(
        cargo_toml
            .get("workspace")?
            .as_table()?
            .get("members")?
            .as_array()?
            .iter()
            .filter_map(|member| member.as_str())
            .filter_map(|member| {
                let member_path = workspace_path.join(member).join("Cargo.toml");
                let member_contents = read_to_string(&member_path).ok()?;
                toml::from_str::<CargoPackage>(&member_contents)
                    .ok()
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
            .collect(),
    )
}
