use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use miette::Diagnostic;
use serde::Deserialize;
use thiserror::Error;
use toml::Spanned;

use super::semver::Version;
use crate::{dry_run::DryRun, fs};

pub(crate) fn get_version(content: &str, path: &Path) -> Result<Version, Error> {
    toml::from_str::<Cargo>(content)
        .map(|cargo| cargo.package.version.into_inner())
        .map_err(|source| Error::Deserialize {
            path: path.into(),
            source,
        })
        .and_then(|version| Version::from_str(&version).map_err(Error::from))
}

pub(crate) fn set_version(
    dry_run: DryRun,
    mut cargo_toml: String,
    new_version: &Version,
    path: &Path,
) -> Result<String, Error> {
    let doc: Cargo = toml::from_str(&cargo_toml).map_err(|source| Error::Deserialize {
        path: path.into(),
        source,
    })?;

    // Account for quotes with +- 1
    let start = doc.package.version.span().start + 1;
    let end = doc.package.version.span().end - 1;

    let version_str = new_version.to_string();
    cargo_toml.replace_range(start..end, &version_str);
    fs::write(dry_run, &version_str, path, &cargo_toml)?;

    Ok(cargo_toml)
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error("Error deserializing {path}: {source}")]
    #[diagnostic(
        code(cargo::deserialize),
        help("knope expects the Cargo.toml file to have a `package.version` property. Workspace support is coming soon!"),
        url("https://knope.tech/reference/config-file/packages/#cargotoml")
    )]
    Deserialize {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Semver(#[from] super::semver::version::Error),
}

#[derive(Debug, Deserialize)]
struct Cargo {
    package: Package,
}

#[derive(Debug, Deserialize)]
struct Package {
    version: Spanned<String>,
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::dry_run::fake_dry_run;

    #[test]
    fn test_get_version() {
        let content = r#"
        [package]
        name = "tester"
        version = "0.1.0-rc.0"
        "#;

        assert_eq!(
            get_version(content, Path::new("")).unwrap(),
            Version::from_str("0.1.0-rc.0").unwrap()
        );
    }

    #[test]
    fn test_set_version() {
        let content = r#"
        [package]
        name = "tester"
        version = "0.1.0-rc.0"
        "#;

        let new = set_version(
            &mut fake_dry_run(),
            String::from(content),
            &Version::from_str("1.2.3-rc.4").unwrap(),
            Path::new(""),
        )
        .unwrap();

        let expected = content.replace("0.1.0-rc.0", "1.2.3-rc.4");
        assert_eq!(new, expected);
    }
}
