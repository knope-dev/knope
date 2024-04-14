use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use toml::Spanned;

use super::package::Package;
use crate::{step::releases::package::PackageName, workflow::Workflow};

/// Loads a `crate::Config` from a TOML file with as much span information as possible for better
/// error messages.
#[derive(Debug, Deserialize)]
pub(crate) struct ConfigLoader {
    pub(crate) package: Option<Spanned<Package>>,
    pub(crate) packages: Option<IndexMap<PackageName, Spanned<Package>>>,
    /// The list of defined workflows that are selectable
    pub(crate) workflows: Option<Spanned<Vec<Spanned<Workflow>>>>,
    /// Optional configuration for Jira
    pub(crate) jira: Option<Spanned<Jira>>,
    /// Optional configuration to talk to GitHub
    pub(crate) github: Option<Spanned<GitHub>>,
    /// Optional configuration to talk to a Gitea instance
    pub(crate) gitea: Option<Spanned<Gitea>>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
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

    #[test]
    fn single_package_with_help() {
        let toml_str =
            format!("[package]{REQUIRED_CONFIG_STUFF}\nhelp_text = \"This is a help text\"");
        let config: ConfigLoader = toml::from_str(&toml_str).unwrap();
        assert!(config.package.is_some());
        assert!(config.packages.is_none());
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

/// Details needed to use steps that interact with a Gitea instance.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub(crate) struct Gitea {
    /// The user or organization that owns the `repo`.
    pub(crate) owner: String,
    /// The name of the repository
    pub(crate) repo: String,
    /// The domain or IP of the Gitea instance
    pub(crate) host: String,
}

impl Gitea {
    /// This lists all known gitea hosts, so we can easily generate the gitea config
    pub(crate) const KNOWN_PUBLIC_GITEA_HOSTS: &'static [&'static str] = &["codeberg.org"];

    fn get_base_url(&self) -> String {
        format!("{host}/api/v1", host = self.host)
    }

    /// get the base url to create and list PRs
    pub(crate) fn get_pulls_url(&self) -> String {
        format!(
            "{base_url}/repos/{owner}/{repo}/pulls",
            base_url = self.get_base_url(),
            owner = self.owner,
            repo = self.repo
        )
    }

    /// get the URL to read/update a single PR
    pub(crate) fn get_pull_url(&self, pr_number: u32) -> String {
        format!("{pulls_url}/{pr_number}", pulls_url = self.get_pulls_url())
    }

    /// Get the URL to create/read releases
    pub(crate) fn get_releases_url(&self) -> String {
        format!(
            "{base_url}/repos/{owner}/{repo}/releases",
            base_url = self.get_base_url(),
            owner = self.owner,
            repo = self.repo
        )
    }

    /// Get the URL to list repo issues
    pub(crate) fn get_issues_url(&self) -> String {
        format!(
            "{base_url}/repos/{owner}/{repo}/issues",
            base_url = self.get_base_url(),
            owner = self.owner,
            repo = self.repo
        )
    }

    /// try to build [Gitea] from a remote formatted like `git@{host}:/{owner}/{repo}`
    /// or `https://{host}/{owner}/{repo}`
    ///
    /// returns None if the remote isn't formatted correctly
    pub(crate) fn try_from_remote(remote: &str) -> Option<Self> {
        // gives [git@something, /x/y.git]
        // or [http(s), host/x/y.git]
        let (scheme, path) = remote.split_once(':')?;

        if scheme.contains("git@") {
            // ssh remote
            // owner/repo.git -> [owner, repo.git]
            let (owner, repo) = path.strip_prefix('/').unwrap_or(path).split_once('/')?;

            Some(Self {
                owner: owner.to_string(),
                repo: repo.strip_suffix(".git").unwrap_or(repo).to_string(),
                host: format!("https://{host}", host = scheme.strip_prefix("git@")?),
            })
        } else {
            // HTTP(s) remote
            // host/owner/repo -> [host, owner, repo]
            let [host, owner, repo]: [&str; 3] = path
                .strip_prefix("//")?
                .splitn(3, '/')
                .collect_vec()
                .try_into()
                .ok()?;

            Some(Self {
                host: format!("https://{host}"),
                owner: owner.to_string(),
                repo: repo.strip_suffix(".git").unwrap_or(repo).to_string(),
            })
        }
    }
}

#[cfg(test)]
mod test_gitea_try_from_remote {
    use super::Gitea;

    #[test]
    fn https_remote() {
        let config = Gitea::try_from_remote("https://codeberg.org/knope-dev/knope.git");
        assert_eq!(
            Some(Gitea {
                owner: "knope-dev".to_string(),
                repo: "knope".to_string(),
                host: "https://codeberg.org".to_string()
            }),
            config
        );
    }

    #[test]
    fn https_remote_without_git_suffix() {
        let config = Gitea::try_from_remote("https://codeberg.org/knope-dev/knope");
        assert_eq!(
            Some(Gitea {
                owner: "knope-dev".to_string(),
                repo: "knope".to_string(),
                host: "https://codeberg.org".to_string()
            }),
            config
        );
    }

    #[test]
    fn ssh_remote() {
        let config = Gitea::try_from_remote("git@codeberg.org:/knope-dev/knope.git");
        assert_eq!(
            Some(Gitea {
                owner: "knope-dev".to_string(),
                repo: "knope".to_string(),
                host: "https://codeberg.org".to_string()
            }),
            config
        );
    }

    #[test]
    fn ssh_remote_without_git_suffix() {
        let config = Gitea::try_from_remote("git@codeberg.org:/knope-dev/knope");
        assert_eq!(
            Some(Gitea {
                owner: "knope-dev".to_string(),
                repo: "knope".to_string(),
                host: "https://codeberg.org".to_string()
            }),
            config
        );
    }
}
