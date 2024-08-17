use std::{ops::Range, path::PathBuf, str::FromStr};

use ::toml::Spanned;
use itertools::Itertools;
use knope_config::{Asset, ChangelogSection};
use knope_versioning::{package, versioned_file::cargo, FormatError, VersionedFilePath};
use miette::Diagnostic;
use relative_path::{RelativePath, RelativePathBuf};
use thiserror::Error;
use toml_edit::{DocumentMut, TomlError};

use crate::{fs, fs::read_to_string};

/// Represents a single package in `knope.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Package {
    pub(crate) name: package::Name,
    /// The files which define the current version of the package.
    pub(crate) versioned_files: Vec<VersionedFilePath>,
    /// The path to the `CHANGELOG.md` file (if any) to be updated when running [`Step::PrepareRelease`].
    pub(crate) changelog: Option<RelativePathBuf>,
    /// Optional scopes that can be used to filter commits when running [`Step::PrepareRelease`].
    pub(crate) scopes: Option<Vec<String>>,
    /// Extra sections that should be added to the changelog from custom footers in commit messages
    /// or change set types.
    pub(crate) extra_changelog_sections: Vec<ChangelogSection>,
    pub(crate) assets: Option<Vec<Asset>>,
    pub(crate) ignore_go_major_versioning: bool,
}

impl Package {
    pub(crate) fn find_in_working_dir() -> Result<Vec<Self>, Error> {
        let packages = Self::cargo_workspace_members()?;

        if !packages.is_empty() {
            return Ok(packages);
        }

        let default_changelog_path = RelativePathBuf::from("CHANGELOG.md");
        let changelog = default_changelog_path
            .to_path("")
            .exists()
            .then_some(default_changelog_path);

        let versioned_files = VersionedFilePath::defaults()
            .into_iter()
            .filter_map(|file_name| {
                let path = file_name.as_path();
                if path.to_path("").exists() {
                    Some(file_name)
                } else {
                    None
                }
            })
            .collect_vec();
        if versioned_files.is_empty() {
            Ok(vec![])
        } else {
            Ok(vec![Self {
                versioned_files,
                changelog,
                ..Self::default()
            }])
        }
    }

    fn cargo_workspace_members() -> Result<Vec<Self>, CargoWorkspaceError> {
        let path = RelativePath::new("Cargo.toml");
        let Ok(contents) = read_to_string(path.as_str()) else {
            return Ok(Vec::new());
        };
        let cargo_toml = DocumentMut::from_str(&contents)
            .map_err(|err| CargoWorkspaceError::Toml(err, path.into()))?;
        let workspace_path = path
            .parent()
            .ok_or_else(|| CargoWorkspaceError::Parent(path.into()))?;
        let Some(members) = cargo_toml
            .get("workspace")
            .and_then(|workspace| workspace.as_table()?.get("members")?.as_array())
        else {
            return Ok(Vec::new());
        };
        let members: Vec<WorkspaceMember> = members
            .iter()
            .map(|member_val| {
                let member = member_val.as_str().ok_or(CargoWorkspaceError::Members)?;
                let member_path =
                    VersionedFilePath::new(workspace_path.join(member).join("Cargo.toml"), None)?;
                let member_contents = read_to_string(member_path.as_path().to_path("."))?;
                let document = DocumentMut::from_str(&member_contents)
                    .map_err(|err| CargoWorkspaceError::Toml(err, member_path.as_path()))?;
                let name = cargo::name_from_document(&document)
                    .ok_or_else(|| CargoWorkspaceError::NoPackageName(member_path.as_path()))?;
                Ok(WorkspaceMember {
                    path: member_path,
                    name: name.to_string(),
                    document,
                })
            })
            .collect::<Result<_, CargoWorkspaceError>>()?;
        // TODO: workspace dependencies
        Ok(members
            .iter()
            .map(|member| {
                let versioned_files = members
                    .iter()
                    .filter_map(|other_member| {
                        if member.name == other_member.name {
                            Some(other_member.path.clone())
                        } else if cargo::contains_dependency(&other_member.document, &member.name) {
                            let mut path = other_member.path.clone();
                            path.dependency = Some(member.name.clone());
                            Some(path)
                        } else {
                            None
                        }
                    })
                    .collect();
                Self {
                    name: package::Name::Custom(member.name.clone()),
                    versioned_files,
                    scopes: Some(vec![member.name.clone()]),
                    changelog: None,
                    extra_changelog_sections: vec![],
                    assets: None,
                    ignore_go_major_versioning: false,
                }
            })
            .collect())
    }

    pub(crate) fn from_toml(
        name: package::Name,
        package: knope_config::Package,
        source_code: &str,
    ) -> Result<Self, VersionedFileError> {
        let knope_config::Package {
            versioned_files,
            changelog,
            scopes,
            extra_changelog_sections,
            assets,
            ignore_go_major_versioning,
        } = package;
        let versioned_files = versioned_files
            .into_iter()
            .map(|spanned| {
                let span = spanned.span();
                VersionedFilePath::try_from(spanned.into_inner())
                    .map_err(|source| VersionedFileError::Format {
                        source,
                        span: span.clone(),
                        source_code: source_code.to_string(),
                    })
                    .and_then(|path| {
                        let pathbuf = path.to_pathbuf();
                        if pathbuf.exists() {
                            Ok(path)
                        } else {
                            Err(VersionedFileError::Missing {
                                path: pathbuf,
                                span,
                                source_code: source_code.to_string(),
                            })
                        }
                    })
            })
            .try_collect()?;
        Ok(Self {
            name,
            versioned_files,
            changelog,
            scopes,
            extra_changelog_sections,
            assets,
            ignore_go_major_versioning,
        })
    }
}

impl From<Package> for knope_config::Package {
    fn from(package: Package) -> Self {
        Self {
            versioned_files: package
                .versioned_files
                .into_iter()
                .map(|it| Spanned::new(0..0, knope_config::VersionedFile::from(it)))
                .collect(),
            changelog: package.changelog,
            scopes: package.scopes,
            extra_changelog_sections: package.extra_changelog_sections,
            assets: package.assets,
            ignore_go_major_versioning: package.ignore_go_major_versioning,
        }
    }
}

#[derive(Debug)]
struct WorkspaceMember {
    path: VersionedFilePath,
    name: String,
    document: DocumentMut,
}

#[derive(Debug, Diagnostic, Error)]
pub enum VersionedFileError {
    #[error("Problem with versioned file")]
    #[diagnostic()]
    Format {
        #[diagnostic_source]
        source: FormatError,
        #[source_code]
        source_code: String,
        #[label("Declared here")]
        span: Range<usize>,
    },
    #[error("File {path} does not exist")]
    #[diagnostic(
        code(config::missing_versioned_file),
        help("Make sure the file exists and is accessible.")
    )]
    Missing {
        path: PathBuf,
        #[source_code]
        source_code: String,
        #[label("Declared here")]
        span: Range<usize>,
    },
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum CargoWorkspaceError {
    #[error("Could not find a package.name in {0}")]
    #[diagnostic(code(workspace::no_package_name))]
    NoPackageName(RelativePathBuf),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
    #[error("Could not parse TOML in {1}: {0}")]
    #[diagnostic(code(workspace::toml))]
    Toml(TomlError, RelativePathBuf),
    #[error("Could not get parent directory of Cargo.toml file: {0}")]
    #[diagnostic(code(workspace::parent))]
    Parent(RelativePathBuf),
    #[error("The Cargo workspace members array should contain only strings")]
    #[diagnostic(code(workspace::members))]
    Members,
    #[error(transparent)]
    #[diagnostic(transparent)]
    UnknownFile(#[from] FormatError),
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    CargoWorkspace(#[from] CargoWorkspaceError),
}
