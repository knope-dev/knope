use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use toml::Spanned;

use super::package::Package;
use crate::{step::releases::package::PackageName, workflow::Workflow};

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
    /// Optional configuration to talk to a Gitea instance
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) gitea: Option<Spanned<Gitea>>,
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

    /// try to build [Gitea] from a remote formatted like `git@{host}:/{owner}/{repo}`
    /// or `https://{host}/{owner}/{repo}`
    ///
    /// returns None if the remote isn't formatted correctly
    pub(crate) fn try_from_remote(remote: &str) -> Option<Self> {
        // gives [git@something, /x/y.git]
        // or [http(s), host/x/y.git]
        let mut split_remote = remote.split(':');

        split_remote.next().and_then(|part| {
            // ssh remote
            if part.contains("git@") {
                // owner/repo.git -> [owner, repo.git]
                let path = split_remote.next()?;
                let mut split_path = path.strip_prefix('/').unwrap_or(path).split('/');

                Some(Self {
                    owner: split_path.next()?.to_string(),
                    // technically a remote should end in .git
                    // but this may not always be the case, so this just makes
                    // sure that we account for that, in case it happens
                    repo: split_path.next()?.split('.').next()?.to_string(),
                    host: format!("https://{host}", host = part.strip_prefix("git@")?),
                })

            // HTTP(s) remote
            } else {
                // host/owner/repo -> [host, owner, repo]
                let mut split_parts = split_remote.next()?.strip_prefix("//")?.split('/');

                Some(Self {
                    host: format!("https://{host}", host = split_parts.next()?),
                    owner: split_parts.next()?.to_string(),
                    // technically a remote should end in .git
                    // but this may not always be the case, so this just makes
                    // sure that we account for that, in case it happens
                    repo: split_parts.next()?.split('.').next()?.to_string(),
                })
            }
        })
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
