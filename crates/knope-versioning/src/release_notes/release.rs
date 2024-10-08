use crate::{package, semver::Version};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Release {
    /// The title of the release without any markdown header prefix
    pub title: String,
    pub version: Version,
    /// The full release notes in Markdown, at header level 1
    pub notes: String,
    pub package_name: package::Name,
}
