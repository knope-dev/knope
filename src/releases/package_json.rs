use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use miette::Diagnostic;
use serde::Deserialize;
use serde_json::{Map, Value};
use thiserror::Error;

use crate::{dry_run::DryRun, fs, releases, releases::semver::Version};

pub(crate) fn get_version(content: &str, path: &Path) -> Result<Version, Error> {
    serde_json::from_str::<Package>(content)
        .map(|package| package.version)
        .map_err(|source| Error::Deserialize {
            path: path.into(),
            source,
        })
        .and_then(|version| Version::from_str(&version).map_err(Error::from))
}

pub(crate) fn set_version(
    dry_run: DryRun,
    package_json: &str,
    new_version: &str,
    path: &Path,
) -> Result<String, Error> {
    let mut json = serde_json::from_str::<Map<String, Value>>(package_json).map_err(|source| {
        Error::Deserialize {
            path: path.into(),
            source,
        }
    })?;
    json.insert(
        "version".to_string(),
        Value::String(new_version.to_string()),
    );
    let contents = serde_json::to_string_pretty(&json).map_err(|source| Error::Serialize {
        path: path.into(),
        source,
    })?;
    fs::write(dry_run, new_version, path, &contents)?;
    Ok(contents)
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error("Error deserializing {path}: {source}")]
    #[diagnostic(
        code(package_json::deserialize),
        help("knope expects the package.json file to be an object with a top level `version` property"),
        url("https://knope-dev.github.io/knope/config/packages.html#supported-formats-for-versioning")
    )]
    Deserialize {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
    #[error("Failed to serialize {path} with new version")]
    #[diagnostic(
        code(package_json::serialize),
        help("This is likely a bug, please report it at https://github.com/knope-dev/knope")
    )]
    Serialize {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error(transparent)]
    #[diagnostic(transparent)]
    Version(#[from] releases::semver::version::Error),
}

#[derive(Debug, Deserialize)]
struct Package {
    version: String,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::dry_run::fake_dry_run;

    #[test]
    fn test_get_version() {
        let content = r#"{
        "name": "tester",
        "version": "0.1.0-rc.0"
        }"#;

        assert_eq!(
            get_version(content, Path::new("")).unwrap(),
            Version::from_str("0.1.0-rc.0").unwrap()
        );
    }

    #[test]
    fn test_set_version() {
        let content = r#"{
        "name": "tester",
        "version": "0.1.0-rc.0"
        }"#;

        let new = set_version(&mut fake_dry_run(), content, "1.2.3-rc.4", Path::new("")).unwrap();

        let expected = r#"{
  "name": "tester",
  "version": "1.2.3-rc.4"
}"#
        .to_string();
        assert_eq!(new, expected);
    }

    #[test]
    fn retain_property_order() {
        let content = r#"{
        "name": "tester",
        "version": "0.1.0-rc.0",
        "dependencies": {}
        }"#;

        let new = set_version(&mut fake_dry_run(), content, "1.2.3-rc.4", Path::new("")).unwrap();

        let expected = r#"{
  "name": "tester",
  "version": "1.2.3-rc.4",
  "dependencies": {}
}"#
        .to_string();
        assert_eq!(new, expected);
    }
}
