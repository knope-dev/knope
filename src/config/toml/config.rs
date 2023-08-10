use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use toml::Spanned;

use super::package::Package;
use crate::{releases::PackageName, workflow::Workflow};

/// Loads a `crate::Config` from a TOML file with as much span information as possible for better
/// error messages.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ConfigLoader {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) package: Option<Spanned<Package>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) packages: Option<IndexMap<PackageName, Spanned<Package>>>,
    /// The list of defined workflows that are selectable
    pub(crate) workflows: Spanned<Vec<Spanned<Workflow>>>,
    /// Optional configuration for Jira
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) jira: Option<Spanned<Jira>>,
    /// Optional configuration to talk to GitHub
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) github: Option<Spanned<GitHub>>,
}

#[cfg(test)]
mod test_package_configs {

    use pretty_assertions::assert_eq;

    use super::ConfigLoader;

    const REQUIRED_CONFIG_STUFF: &str = "\n[[workflows]]\nname = \"default\"\n[[workflows.steps]]\ntype = \"Command\"\ncommand = \"echo this is nothing, really\"";

    #[test]
    fn single_package() {
        let toml_str = format!("[package]{REQUIRED_CONFIG_STUFF}");
        let config: ConfigLoader = toml::from_str(&toml_str).unwrap();
        assert!(config.package.is_some());
        assert!(config.packages.is_none());
    }

    #[test]
    fn multi_package() {
        let toml_str = format!("[packages.something]\n[packages.blah]{REQUIRED_CONFIG_STUFF}");
        let config: ConfigLoader = toml::from_str(&toml_str).unwrap();
        assert!(config.package.is_none());
        let packages = config.packages.unwrap();
        assert_eq!(packages.len(), 2);
        assert!(packages.contains_key("something"));
        assert!(packages.contains_key("blah"));
    }
}

/// Config required for steps that interact with Jira.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct Jira {
    /// The URL to your Atlassian instance running Jira
    pub(crate) url: String,
    /// The key of the Jira project to filter on (the label of all issues)
    pub(crate) project: String,
}

/// Details needed to use steps that interact with GitHub.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct GitHub {
    /// The user or organization that owns the `repo`.
    pub(crate) owner: String,
    /// The name of the repository in GitHub that this project is utilizing
    pub(crate) repo: String,
}
