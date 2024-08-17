use indexmap::IndexMap;
use knope_versioning::{release_notes::Release, semver::Version, Action};
use miette::Diagnostic;
use serde::{Deserialize, Serialize};

use crate::{
    integrations::git::branch_name_from_issue,
    state,
    state::State,
    step::releases::{package, semver, Package},
};

/// Describes a value that can replace an arbitrary string in certain steps.
///
/// <https://knope.tech/reference/config-file/variables//>
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(crate) enum Variable {
    /// The version of the package, if only a single package is configured (error if multiple).
    Version,
    /// The generated branch name for the selected issue. Note that this means the workflow must
    /// already be in [`State::IssueSelected`] when this variable is used.
    IssueBranch,
    /// Get the current changelog entry from the latest release.
    ChangelogEntry,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// A template string and the variables that should be replaced in it.
pub(crate) struct Template {
    pub(crate) template: String,
    #[serde(default)]
    pub(crate) variables: IndexMap<String, Variable>,
}

/// Replace declared variables in the string and return the new string.
pub(crate) fn replace_variables(template: Template, state: &mut State) -> Result<String, Error> {
    let mut package_cache = None;
    let Template {
        mut template,
        variables,
    } = template;
    for (var_name, var_type) in variables {
        match var_type {
            Variable::Version => {
                let package = if let Some(package) = package_cache.take() {
                    package
                } else {
                    first_package(state)?
                };
                let version = package.versioning.versions.clone().into_latest();
                template = template.replace(&var_name, &version.to_string());
                package_cache = Some(package);
            }
            Variable::ChangelogEntry => {
                let package = if let Some(package) = package_cache.take() {
                    package
                } else {
                    first_package(state)?
                };
                if let Some(body) = package.pending_actions.iter().find_map(|action| {
                    if let Action::CreateRelease(Release { notes, .. }) = action {
                        Some(notes)
                    } else {
                        None
                    }
                }) {
                    template = template.replace(&var_name, body);
                } else {
                    let version = package.versioning.versions.clone().into_latest();
                    let release = package
                        .versioning
                        .release_notes
                        .changelog
                        .as_ref()
                        .and_then(|changelog| changelog.get_release(&version))
                        .ok_or_else(|| Error::NoChangelogEntry(version))?;
                    template = template.replace(&var_name, &release.notes);
                }
                package_cache = Some(package);
            }
            Variable::IssueBranch => match &state.issue {
                state::Issue::Initial => return Err(Error::NoIssueSelected),
                state::Issue::Selected(issue) => {
                    template = template.replace(&var_name, &branch_name_from_issue(issue));
                }
            },
        }
    }
    if let Some(package) = package_cache {
        state.packages.push(package);
    }
    Ok(template)
}

fn first_package(state: &mut State) -> Result<Package, Error> {
    if state.packages.len() > 1 {
        Err(Error::TooManyPackages)
    } else if let Some(package) = state.packages.pop() {
        Ok(package)
    } else {
        Err(package::Error::NoDefinedPackages.into())
    }
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("Too many packages defined")]
    #[diagnostic(
        code(variables::too_many_packages),
        help("The Version and Changelog variables can only be used with a single [package].")
    )]
    TooManyPackages,
    #[error(transparent)]
    #[diagnostic(transparent)]
    Package(#[from] package::Error),
    #[error("Could not find a changelog entry for version {0}")]
    #[diagnostic(
        code(variables::no_changelog_entry),
        url("https://knope.tech/reference/concepts/changelog/#versions")
    )]
    NoChangelogEntry(Version),
    #[error("No issue selected")]
    #[diagnostic(
        code(variables::no_issue_selected),
        help("The IssueBranch command variable requires selecting an issue first with SelectGitHubIssue or SelectJiraIssue")
    )]
    NoIssueSelected,
    #[error(transparent)]
    #[diagnostic(transparent)]
    SemVer(#[from] semver::Error),
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::indexing_slicing)]
mod test_replace_variables {
    use knope_versioning::{
        package::Name,
        release_notes::{Changelog, ReleaseNotes, Sections},
        Action, VersionedFile, VersionedFilePath,
    };
    use pretty_assertions::assert_eq;
    use relative_path::RelativePathBuf;

    use super::*;
    use crate::step::issues::Issue;

    fn package() -> Package {
        let changelog = Changelog::new(RelativePathBuf::default(), String::new());

        Package {
            versioning: knope_versioning::Package::new(
                Name::Default,
                &[""],
                vec![VersionedFile::new(
                    VersionedFilePath::new("Cargo.toml".into(), None).unwrap(),
                    "[package]\nversion = \"1.2.3\"\nname=\"blah\"".into(),
                    &[""],
                )
                .unwrap()],
                ReleaseNotes {
                    sections: Sections::default(),
                    changelog: Some(changelog),
                },
                None,
            )
            .unwrap(),
            ..Package::default()
        }
    }

    #[test]
    fn replace_prepared_version() {
        let template = "blah $$ other blah".to_string();
        let mut variables = IndexMap::new();
        variables.insert("$$".to_string(), Variable::Version);
        let mut state = State::new(None, None, None, vec![package()], Vec::new());
        let version = Version::new(1, 2, 3, None);
        let package_versions = version.clone().into();
        state.packages[0].versioning.versions = package_versions;

        let result = replace_variables(
            Template {
                template,
                variables,
            },
            &mut state,
        )
        .unwrap();

        assert_eq!(result, format!("blah {version} other blah"));
    }

    #[test]
    fn replace_issue_branch() {
        let template = "blah $$ other blah".to_string();
        let mut variables = IndexMap::new();
        variables.insert("$$".to_string(), Variable::IssueBranch);
        let issue = Issue {
            key: "13".to_string(),
            summary: "1234".to_string(),
        };
        let expected_branch_name = branch_name_from_issue(&issue);
        let mut state = State {
            jira_config: None,
            github: state::GitHub::New,
            github_config: None,
            gitea: state::Gitea::New,
            gitea_config: None,
            issue: state::Issue::Selected(issue),
            packages: Vec::new(),
            all_git_tags: Vec::new(),
        };

        let result = replace_variables(
            Template {
                template,
                variables,
            },
            &mut state,
        )
        .unwrap();

        assert_eq!(result, format!("blah {expected_branch_name} other blah"));
    }

    #[test]
    fn replace_changelog_entry_prepared_release() {
        let template = "blah $$ other blah".to_string();
        let mut variables = IndexMap::new();
        variables.insert("$$".to_string(), Variable::ChangelogEntry);
        let mut state = State::new(None, None, None, vec![package()], Vec::new());
        let version = Version::new(1, 2, 3, None);
        let changelog_entry = "some content being put in the changelog";
        state.packages[0].pending_actions = vec![Action::CreateRelease(Release {
            version: version.clone(),
            title: "title".to_string(),
            notes: changelog_entry.to_string(),
        })];

        let result = replace_variables(
            Template {
                template,
                variables,
            },
            &mut state,
        )
        .unwrap();

        assert_eq!(result, format!("blah {changelog_entry} other blah"));
    }
}
