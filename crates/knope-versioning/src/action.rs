use std::str::FromStr;

use relative_path::RelativePathBuf;

use crate::{package, release_notes::Release, semver::Version};

/// Actions to take to finish updating a package
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    WriteToFile {
        path: RelativePathBuf,
        content: String,
        diff: String,
    },
    RemoveFile {
        path: RelativePathBuf,
    },
    AddTag {
        tag: String,
    },
    CreateRelease(Release),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseTag(String);

impl ReleaseTag {
    /// The tag that a particular version should have for a particular package
    #[must_use]
    pub fn new(version: &Version, package_name: &package::Name) -> Self {
        let prefix = Self::tag_prefix(package_name);
        Self(format!("{prefix}{version}"))
    }

    #[must_use]
    pub fn is_release_tag(val: &str, package_name: &package::Name) -> bool {
        let tag_prefix = Self::tag_prefix(package_name);
        val.strip_prefix(&tag_prefix)
            .and_then(|version_str| Version::from_str(version_str).ok())
            .is_some()
    }

    /// The prefix for tags for a particular package
    fn tag_prefix(package_name: &package::Name) -> String {
        package_name
            .as_custom()
            .as_ref()
            .map_or_else(|| "v".to_string(), |name| format!("{name}/v"))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<ReleaseTag> for String {
    fn from(tag: ReleaseTag) -> Self {
        tag.0
    }
}

pub(crate) enum ActionSet {
    Single(Action),
    Two([Action; 2]),
}

impl IntoIterator for ActionSet {
    type Item = Action;
    type IntoIter = ActionSetIter;

    fn into_iter(self) -> Self::IntoIter {
        ActionSetIter {
            actions: Some(self),
        }
    }
}

#[allow(clippy::module_name_repetitions)]
pub struct ActionSetIter {
    actions: Option<ActionSet>,
}

impl Iterator for ActionSetIter {
    type Item = Action;

    fn next(&mut self) -> Option<Self::Item> {
        match self.actions.take() {
            None => None,
            Some(ActionSet::Single(action)) => {
                self.actions = None;
                Some(action)
            }
            Some(ActionSet::Two([first, second])) => {
                self.actions = None;
                self.actions = Some(ActionSet::Single(second));
                Some(first)
            }
        }
    }
}
