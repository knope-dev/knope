use std::{fmt::Write, str::FromStr};

use jsonc_parser::{ParseOptions, parse_to_serde_value};
#[cfg(feature = "miette")]
use miette::Diagnostic;
use relative_path::RelativePathBuf;
use serde_json::Value;
use thiserror::Error;

use crate::{action::Action, semver::Version};

#[derive(Clone, Debug)]
pub struct DenoJson {
    path: RelativePathBuf,
    raw: String,
    parsed: Value,
    diff: Option<String>,
}

impl DenoJson {
    pub(crate) fn new(path: RelativePathBuf, content: String) -> Result<Self, Error> {
        let parsed = parse_to_serde_value(&content, &ParseOptions::default())
            .map_err(|source| Error::Parse {
                source,
                path: path.clone(),
            })?
            .ok_or_else(|| Error::NoContent { path: path.clone() })?;

        Ok(DenoJson {
            path,
            raw: content,
            parsed,
            diff: None,
        })
    }

    pub(crate) fn get_version(&self) -> Option<Version> {
        self.parsed
            .get("version")
            .and_then(Value::as_str)
            .and_then(|val| Version::from_str(val).ok())
    }

    pub(crate) fn get_path(&self) -> &RelativePathBuf {
        &self.path
    }

    pub(crate) fn set_version(
        mut self,
        new_version: &Version,
        dependency: Option<&str>,
    ) -> serde_json::Result<Self> {
        if dependency.is_some() {
            // Dependency versions are governed by deno.lock in modern workspaces, so
            // deno.json entries typically omit explicit version specifiers.
            // See: https://docs.deno.com/runtime/reference/workspaces/#lockfile
            return Ok(self);
        }

        self.parsed.as_object_mut().and_then(|obj| {
            obj.insert(
                "version".to_string(),
                Value::String(new_version.to_string()),
            )
        });

        let diff = self.diff.get_or_insert_default();
        if !diff.is_empty() {
            diff.push_str(", ");
        }
        write!(diff, "version = {new_version}").ok();

        self.raw = serde_json::to_string_pretty(&self.parsed)?;
        Ok(self)
    }
}

impl DenoJson {
    pub(super) fn write(self) -> Option<Action> {
        self.diff.map(|diff| Action::WriteToFile {
            path: self.path,
            content: self.raw,
            diff,
        })
    }
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum Error {
    #[error("Could not parse {path}: {source}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::versioned_file::deno_json::parse),
            help("Make sure the file is valid JSON")
        )
    )]
    Parse {
        source: jsonc_parser::errors::ParseError,
        path: RelativePathBuf,
    },
    #[error("File {path} was empty or contained only comments.")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(code(knope_versioning::versioned_file::deno_json::no_content),)
    )]
    NoContent { path: RelativePathBuf },
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_deno_json_with_version() {
        let content = r#"{"name": "@scope/package", "version": "1.0.0"}"#;
        let deno_json = DenoJson::new("deno.json".into(), content.to_string()).unwrap();
        assert_eq!(
            deno_json.get_version(),
            Some(Version::from_str("1.0.0").unwrap())
        );
    }

    #[test]
    fn test_deno_json_without_version() {
        let content = r#"{"name": "@scope/package", "tasks": {"dev": "deno run main.ts"}}"#;
        let deno_json = DenoJson::new("deno.json".into(), content.to_string()).unwrap();
        assert_eq!(deno_json.get_version(), None);
    }

    #[test]
    fn test_set_version() {
        let content = r#"{"name": "@scope/package", "version": "1.0.0"}"#;
        let deno_json = DenoJson::new("deno.json".into(), content.to_string()).unwrap();
        let new_version = Version::from_str("1.1.0").unwrap();
        let updated = deno_json.set_version(&new_version, None).unwrap();
        assert_eq!(updated.get_version(), Some(new_version));
    }

    #[test]
    fn test_set_version_on_file_without_version() {
        let content = r#"{"name": "@scope/package", "tasks": {"dev": "deno run main.ts"}}"#;
        let deno_json = DenoJson::new("deno.json".into(), content.to_string()).unwrap();
        let new_version = Version::from_str("1.0.0").unwrap();
        let updated = deno_json.set_version(&new_version, None).unwrap();
        assert_eq!(updated.get_version(), Some(new_version));
    }

    #[test]
    fn test_deno_json_with_comments() {
        let content = "// leading comment\n{\"name\": \"@scope/package\", \"version\": \"1.0.0\"}";
        let deno_json = DenoJson::new("deno.jsonc".into(), content.to_string()).unwrap();
        assert_eq!(
            deno_json.get_version(),
            Some(Version::from_str("1.0.0").unwrap())
        );
    }

    #[test]
    fn test_set_version_with_comment_source() {
        let content = "// leading comment\n{\"name\": \"@scope/package\", \"version\": \"1.0.0\"}";
        let deno_json = DenoJson::new("deno.jsonc".into(), content.to_string()).unwrap();
        let new_version = Version::from_str("1.2.0").unwrap();
        let updated = deno_json.set_version(&new_version, None).unwrap();
        assert_eq!(updated.get_version(), Some(new_version));
    }
}
