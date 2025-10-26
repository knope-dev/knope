#[cfg(feature = "miette")]
use miette::Diagnostic;
use regex::Regex;
use relative_path::RelativePathBuf;
use thiserror::Error;

use crate::{Action, semver::Version};

#[derive(Clone, Debug)]
pub struct TextFile {
    pub(super) path: RelativePathBuf,
    content: String,
    pattern: String,
    diff: Option<String>,
}

impl TextFile {
    /// Creates a new TextFile with the given pattern.
    ///
    /// # Errors
    ///
    /// If the regex pattern is invalid
    pub fn new(path: RelativePathBuf, content: String, pattern: String) -> Result<Self, Error> {
        // Validate the regex pattern
        Regex::new(&pattern).map_err(|source| Error::InvalidPattern {
            pattern: pattern.clone(),
            path: path.clone(),
            source,
        })?;

        Ok(Self {
            path,
            content,
            pattern,
            diff: None,
        })
    }

    /// Get the current version from the file using the regex pattern.
    ///
    /// # Errors
    ///
    /// If the pattern doesn't match or the matched version is invalid
    pub(super) fn get_version(&self) -> Result<Version, Error> {
        let re = Regex::new(&self.pattern).map_err(|source| Error::InvalidPattern {
            pattern: self.pattern.clone(),
            path: self.path.clone(),
            source,
        })?;

        let caps = re.captures(&self.content).ok_or_else(|| Error::NoMatch {
            pattern: self.pattern.clone(),
            path: self.path.clone(),
        })?;

        // Try to get the first capture group, or use the full match if no groups
        let version_str = caps
            .get(1)
            .or_else(|| caps.get(0))
            .ok_or_else(|| Error::NoMatch {
                pattern: self.pattern.clone(),
                path: self.path.clone(),
            })?
            .as_str();

        version_str.parse().map_err(|err| Error::InvalidVersion {
            version: version_str.to_string(),
            path: self.path.clone(),
            source: err,
        })
    }

    /// Set the version in the file using the regex pattern.
    #[must_use]
    pub(super) fn set_version(mut self, new_version: &Version) -> Self {
        if let Ok(re) = Regex::new(&self.pattern) {
            let new_version_str = new_version.to_string();
            let old_content = self.content.clone();
            
            // Replace using the pattern - if there's a capture group, replace only that
            self.content = re.replace(&self.content, |caps: &regex::Captures| {
                if caps.len() > 1 {
                    // There's a capture group - replace only the captured part
                    let full_match = caps.get(0).map_or("", |m| m.as_str());
                    let captured = caps.get(1).map_or("", |m| m.as_str());
                    full_match.replace(captured, &new_version_str)
                } else {
                    // No capture group - replace the full match
                    new_version_str.clone()
                }
            }).to_string();

            // Create a simple diff for display
            if let Some(changed_line) = old_content
                .lines()
                .zip(self.content.lines())
                .find(|(old, new)| old != new)
                .map(|(_, new)| new)
            {
                self.diff = Some(changed_line.to_string());
            }
        }

        self
    }

    pub(super) fn write(self) -> Option<Action> {
        self.diff.map(|diff| Action::WriteToFile {
            path: self.path,
            content: self.content,
            diff,
        })
    }
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum Error {
    #[error("Invalid regex pattern '{pattern}' for {path}: {source}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::text_file::invalid_pattern),
            help("Check the regex pattern syntax"),
            url("https://docs.rs/regex/latest/regex/#syntax")
        )
    )]
    InvalidPattern {
        pattern: String,
        path: RelativePathBuf,
        #[source]
        source: regex::Error,
    },

    #[error("Pattern '{pattern}' did not match any content in {path}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::text_file::no_match),
            help("Ensure the pattern matches the version string in the file")
        )
    )]
    NoMatch {
        pattern: String,
        path: RelativePathBuf,
    },

    #[error("Matched version '{version}' in {path} is not a valid semantic version: {source}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::text_file::invalid_version),
            help("The matched string must be a valid semantic version")
        )
    )]
    InvalidVersion {
        version: String,
        path: RelativePathBuf,
        #[source]
        source: crate::semver::Error,
    },
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::str::FromStr;

    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_simple_version_pattern() {
        let content = "version: 1.2.3\nother: stuff";
        let pattern = r"version:\s+(\d+\.\d+\.\d+)";
        
        let file = TextFile::new(
            RelativePathBuf::from("test.txt"),
            content.to_string(),
            pattern.to_string(),
        )
        .unwrap();

        let version = file.get_version().unwrap();
        assert_eq!(version, Version::from_str("1.2.3").unwrap());
    }

    #[test]
    fn test_set_version() {
        let content = "version: 1.2.3\nother: stuff";
        let pattern = r"version:\s+(\d+\.\d+\.\d+)";
        
        let file = TextFile::new(
            RelativePathBuf::from("test.txt"),
            content.to_string(),
            pattern.to_string(),
        )
        .unwrap();

        let new_version = Version::from_str("2.0.0").unwrap();
        let updated = file.set_version(&new_version);

        assert!(updated.content.contains("version: 2.0.0"));
        assert!(updated.content.contains("other: stuff"));
    }

    #[test]
    fn test_readme_example() {
        let content = r#"steps:
      - uses: knope-dev/action@v1
        with:
          version: 0.21.4"#;
        let pattern = r"version:\s+(\d+\.\d+\.\d+)";
        
        let file = TextFile::new(
            RelativePathBuf::from("README.md"),
            content.to_string(),
            pattern.to_string(),
        )
        .unwrap();

        let version = file.get_version().unwrap();
        assert_eq!(version, Version::from_str("0.21.4").unwrap());

        let new_version = Version::from_str("0.22.0").unwrap();
        let updated = file.set_version(&new_version);

        assert!(updated.content.contains("version: 0.22.0"));
    }

    #[test]
    fn test_invalid_pattern() {
        let content = "version: 1.2.3";
        let pattern = r"[invalid(regex";
        
        let result = TextFile::new(
            RelativePathBuf::from("test.txt"),
            content.to_string(),
            pattern.to_string(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_pattern_no_match() {
        let content = "no version here";
        let pattern = r"version:\s+(\d+\.\d+\.\d+)";
        
        let file = TextFile::new(
            RelativePathBuf::from("test.txt"),
            content.to_string(),
            pattern.to_string(),
        )
        .unwrap();

        let result = file.get_version();
        assert!(result.is_err());
    }
}
