use knope_config::{Template, Variable};
use knope_versioning::{Action, release_notes::Release, semver::Version};
use miette::Diagnostic;

use crate::{
    integrations::git::branch_name_from_issue,
    state,
    state::State,
    step::releases::{Package, package, semver},
};

/// Replace declared variables in the string and return the new string.
pub(crate) fn replace_variables(template: &Template, state: &mut State) -> Result<String, Error> {
    let mut package_cache = None;
    let template = template.replace_variables(|variable| match variable {
        Variable::Version => {
            let package = if let Some(package) = package_cache.take() {
                package
            } else {
                first_package(state)?
            };
            let version = package.versioning.versions.clone().into_latest();
            package_cache = Some(package);
            Ok(version.to_string())
        }
        Variable::ChangelogEntry => {
            let package = if let Some(package) = package_cache.take() {
                package
            } else {
                first_package(state)?
            };
            if let Some(body) = state.pending_actions.iter().find_map(|action| {
                if let Action::CreateRelease(Release { notes, .. }) = action {
                    Some(notes)
                } else {
                    None
                }
            }) {
                package_cache = Some(package);
                Ok(body.clone())
            } else {
                let version = package.versioning.versions.clone().into_latest();
                let release = package
                    .versioning
                    .release_notes
                    .changelog
                    .as_ref()
                    .and_then(|changelog| changelog.get_release(&version, package.name()))
                    .ok_or_else(|| Error::NoChangelogEntry(version))?;
                package_cache = Some(package);
                Ok(release.notes)
            }
        }
        Variable::IssueBranch => match &state.issue {
            state::Issue::Initial => Err(Error::NoIssueSelected),
            state::Issue::Selected(issue) => Ok(branch_name_from_issue(issue)),
        },
    })?;
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
        help(
            "The IssueBranch command variable requires selecting an issue first with SelectGitHubIssue or SelectJiraIssue"
        )
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
    use std::borrow::Cow;

    use indexmap::IndexMap;
    use knope_versioning::{
        Action, VersionedFile, VersionedFileConfig,
        package::Name,
        release_notes::{Changelog, ReleaseNotes, Sections},
    };
    use pretty_assertions::assert_eq;
    use relative_path::RelativePathBuf;

    use super::*;
    use crate::step::issues::Issue;

    fn state() -> State {
        let changelog = Changelog::new(RelativePathBuf::default(), String::new());
        let versioned_file_path = VersionedFileConfig::new("Cargo.toml".into(), None).unwrap();
        let all_versioned_files = vec![
            VersionedFile::new(
                &versioned_file_path,
                "[package]\nversion = \"1.2.3\"\nname=\"blah\"".into(),
                &[""],
            )
            .unwrap(),
        ];

        let package = Package {
            versioning: knope_versioning::Package::new(
                Name::Default,
                &[""],
                vec![versioned_file_path],
                &all_versioned_files,
                ReleaseNotes {
                    sections: Sections::default(),
                    changelog: Some(changelog),
                },
                None,
            )
            .unwrap(),
            ..Package::default()
        };

        State::new(
            None,
            None,
            None,
            vec![package],
            all_versioned_files,
            Vec::new(),
        )
    }

    #[test]
    fn replace_prepared_version() {
        let template = "blah $$ other blah".to_string();
        let mut variables = IndexMap::new();
        variables.insert(Cow::Borrowed("$$"), Variable::Version);
        let mut state = state();
        let version = Version::new(1, 2, 3, None);
        let package_versions = version.clone().into();
        state.packages[0].versioning.versions = package_versions;

        let result =
            replace_variables(&Template::new(template, Some(variables)), &mut state).unwrap();

        assert_eq!(result, format!("blah {version} other blah"));
    }

    #[test]
    fn replace_issue_branch() {
        let template = "blah $$ other blah".to_string();
        let mut variables = IndexMap::new();
        variables.insert(Cow::Borrowed("$$"), Variable::IssueBranch);
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
            all_versioned_files: Vec::new(),
            pending_actions: Vec::new(),
        };

        let result =
            replace_variables(&Template::new(template, Some(variables)), &mut state).unwrap();

        assert_eq!(result, format!("blah {expected_branch_name} other blah"));
    }

    #[test]
    fn replace_changelog_entry_prepared_release() {
        let template = "blah $changelog other blah".to_string();
        let mut state = state();
        let version = Version::new(1, 2, 3, None);
        let changelog_entry = "some content being put in the changelog";
        state.pending_actions = vec![Action::CreateRelease(Release {
            version: version.clone(),
            title: "title".to_string(),
            notes: changelog_entry.to_string(),
            package_name: Name::Default,
        })];

        let result = replace_variables(&Template::new(template, None), &mut state).unwrap();

        assert_eq!(result, format!("blah {changelog_entry} other blah"));
    }
}
