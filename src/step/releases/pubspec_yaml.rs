use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use serde_yaml::{from_str, to_string, Mapping, Value};
use thiserror::Error;

use crate::{
    dry_run::DryRun,
    fs,
    step::{releases, releases::semver::Version},
};

pub(crate) fn get_version(content: &str, path: &Path) -> Result<Version, Error> {
    from_str::<PubSpec>(content)
        .map(|pub_spec: PubSpec| pub_spec.version)
        .map_err(|source| Error::Deserialize {
            path: path.into(),
            source,
        })
        .and_then(|version| Version::from_str(&version).map_err(Error::from))
}

pub(crate) fn set_version(
    dry_run: DryRun,
    pubspec_yaml: &str,
    new_version: &Version,
    path: &Path,
) -> Result<String, Error> {
    let version_line = pubspec_yaml
        .lines()
        .find(|line| line.starts_with("version: "));
    let contents = if let Some(version_line) = version_line {
        // Replace only the required bit to preserve formatting & comments (since serde_yaml doesn't preserve them)
        let new_version_line = to_string(&PubSpec {
            version: new_version.to_string(),
        })
        .map_err(|source| Error::Serialize {
            path: path.into(),
            source,
        })?;
        pubspec_yaml.replace(version_line, new_version_line.trim())
    } else {
        // Can't replace just the one line, resort to replacing the whole thing
        let mut yaml = from_str::<Mapping>(pubspec_yaml).map_err(|source| Error::Deserialize {
            path: path.into(),
            source,
        })?;
        yaml.insert(
            Value::String("version".to_string()),
            Value::String(new_version.to_string()),
        );
        to_string(&yaml).map_err(|source| Error::Serialize {
            path: path.into(),
            source,
        })?
    };

    fs::write(dry_run, &new_version.to_string(), path, &contents)?;
    Ok(contents)
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error("Error deserializing {path}: {source}")]
    #[diagnostic(
        code(pubspec_yaml::deserialize),
        help("knope expects the pubspec.yaml file to be an object with a top level `version` property"),
        url("https://knope.tech/reference/config-file/packages/#pubspecyaml")
    )]
    Deserialize {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
    #[error("Failed to serialize {path} with new version")]
    #[diagnostic(
        code(pubspec_yaml::serialize),
        help("This is likely a bug, please report it at https://github.com/knope-dev/knope")
    )]
    Serialize {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error(transparent)]
    #[diagnostic(transparent)]
    Version(#[from] releases::semver::version::Error),
}

#[derive(Debug, Deserialize, Serialize)]
struct PubSpec {
    version: String,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use pretty_assertions::assert_eq;

    use super::*;
    use crate::dry_run::fake_dry_run;
    #[test]
    fn test_get_version() {
        let content = include_str!("../../../tests/prepare_release/pubspec_yaml/in/pubspec.yaml");

        assert_eq!(
            get_version(content, Path::new("")).unwrap(),
            Version::from_str("1.0.0").unwrap()
        );
    }

    #[test]
    fn test_set_version() {
        let content = include_str!("../../../tests/prepare_release/pubspec_yaml/in/pubspec.yaml");

        let new = set_version(
            &mut fake_dry_run(),
            content,
            &Version::from_str("1.2.3-rc.4").unwrap(),
            Path::new(""),
        )
        .unwrap();

        let expected = content.replace("version: 1.0.0", "version: 1.2.3-rc.4");
        assert_eq!(new, expected);
    }
}
