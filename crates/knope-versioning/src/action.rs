use relative_path::RelativePathBuf;

use crate::{package, semver::Version};

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
    CreateRelease(CreateRelease),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateRelease {
    // TODO: this should have the title in it...
    pub version: Version,
    pub notes: String,
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
        // TODO: check for version component?
        val.starts_with(&Self::tag_prefix(package_name))
    }

    /// The prefix for tags for a particular package
    ///
    /// TODO: Is this used anywhere other than `new`?
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
