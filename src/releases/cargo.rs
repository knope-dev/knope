use std::path::{Path, PathBuf};

use miette::Diagnostic;
use serde::Deserialize;
use thiserror::Error;
use toml::Spanned;

use crate::fs;

pub(crate) fn get_version(content: &str, path: &Path) -> Result<String, Error> {
    toml::from_str::<Cargo>(content)
        .map(|cargo| cargo.package.version.into_inner())
        .map_err(|source| Error::Deserialize {
            path: path.into(),
            source,
        })
}

pub(crate) fn set_version(
    dry_run: &mut Option<Box<dyn std::io::Write>>,
    mut cargo_toml: String,
    new_version: &str,
    path: &Path,
) -> Result<String, Error> {
    let doc: Cargo = toml::from_str(&cargo_toml).map_err(|source| Error::Deserialize {
        path: path.into(),
        source,
    })?;

    // Account for quotes with +- 1
    let start = doc.package.version.span().start + 1;
    let end = doc.package.version.span().end - 1;

    cargo_toml.replace_range(start..end, new_version);
    fs::write(dry_run, new_version, path, &cargo_toml)?;

    Ok(cargo_toml)
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error("Error deserializing {path}: {source}")]
    #[diagnostic(
        code(cargo::deserialize),
        help("knope expects the Cargo.toml file to have a `package.version` property. Workspace support is coming soon!"),
        url("https://knope-dev.github.io/knope/config/packages.html#supported-formats-for-versioning")
    )]
    Deserialize {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error(transparent)]
    Fs(#[from] fs::Error),
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

    #[test]
    fn test_get_version() {
        let content = r###"
        [package]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;

        assert_eq!(
            get_version(content, Path::new("")).unwrap(),
            "0.1.0-rc.0".to_string()
        );
    }

    #[test]
    fn test_set_version() {
        let content = r###"
        [package]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;

        let stdout = Box::<Vec<u8>>::default();
        let new = set_version(
            &mut Some(stdout),
            String::from(content),
            "1.2.3-rc.4",
            Path::new(""),
        )
        .unwrap();

        let expected = content.replace("0.1.0-rc.0", "1.2.3-rc.4");
        assert_eq!(new, expected);
    }
}
