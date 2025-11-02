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
    regex: Regex,
    diff: Option<String>,
}

impl TextFile {
    /// Creates a new `TextFile` with the given regex pattern.
    ///
    /// # Errors
    ///
    /// If the regex pattern is invalid or doesn't contain a named "version" capture group
    pub fn new(path: RelativePathBuf, content: String, regex: String) -> Result<Self, Error> {
        // Compile and validate the regex pattern
        let re = Regex::new(&regex).map_err(|source| Error::InvalidPattern {
            regex: regex.clone(),
            path: path.clone(),
            source,
        })?;

        // Check that the regex has at least one named capture group called "version"
        if re.capture_names().all(|name| name != Some("version")) {
            return Err(Error::MissingVersionGroup {
                regex,
                path: path.clone(),
            });
        }

        Ok(Self {
            path,
            content,
            regex: re,
            diff: None,
        })
    }

    /// Get the current version from the file using the regex pattern.
    ///
    /// # Errors
    ///
    /// If the pattern doesn't match or the matched version is invalid
    pub(super) fn get_version(&self) -> Result<Version, Error> {
        let caps = self
            .regex
            .captures(&self.content)
            .ok_or_else(|| Error::NoMatch {
                regex: self.regex.as_str().to_string(),
                path: self.path.clone(),
            })?;

        // Get the named "version" capture group
        let version_str = caps
            .name("version")
            .ok_or_else(|| Error::NoMatch {
                regex: self.regex.as_str().to_string(),
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
        let new_version_str = new_version.to_string();
        let old_content = self.content.clone();

        // Replace all named "version" capture groups with the new version
        self.content = self
            .regex
            .replace_all(&self.content, |caps: &regex::Captures| {
                // Get the full match text, then replace the "version" named group within it
                // This preserves any surrounding context while only updating the version number
                let mut result = caps.get(0).map_or("", |m| m.as_str()).to_string();

                // Replace each "version" named group in the match
                if let Some(version_match) = caps.name("version") {
                    result = result.replace(version_match.as_str(), &new_version_str);
                }

                result
            })
            .to_string();

        // Create a simple diff for display
        if let Some(changed_line) = old_content
            .lines()
            .zip(self.content.lines())
            .find(|(old, new)| old != new)
            .map(|(_, new)| new.trim())
        {
            self.diff = Some(changed_line.to_string());
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
    #[error("Invalid regex pattern '{regex}' for {path}: {source}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::text_file::invalid_pattern),
            help("Check the regex pattern syntax"),
            url("https://docs.rs/regex/latest/regex/#syntax")
        )
    )]
    InvalidPattern {
        regex: String,
        path: RelativePathBuf,
        #[source]
        source: regex::Error,
    },

    #[error(
        "Regex pattern '{regex}' must contain at least one named capture group called 'version'"
    )]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::text_file::missing_version_group),
            help("Use a named capture group like (?<version>\\d+\\.\\d+\\.\\d+) in your regex")
        )
    )]
    MissingVersionGroup {
        regex: String,
        path: RelativePathBuf,
    },

    #[error("Regex '{regex}' did not match any content in {path}")]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(knope_versioning::text_file::no_match),
            help("Ensure the regex matches the version string in the file")
        )
    )]
    NoMatch {
        regex: String,
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
        let regex = r"version:\s+(?<version>\d+\.\d+\.\d+)";

        let file = TextFile::new(
            RelativePathBuf::from("test.txt"),
            content.to_string(),
            regex.to_string(),
        )
        .unwrap();

        let version = file.get_version().unwrap();
        assert_eq!(version, Version::from_str("1.2.3").unwrap());
    }

    #[test]
    fn test_set_version() {
        let content = "version: 1.2.3\nother: stuff";
        let regex = r"version:\s+(?<version>\d+\.\d+\.\d+)";

        let file = TextFile::new(
            RelativePathBuf::from("test.txt"),
            content.to_string(),
            regex.to_string(),
        )
        .unwrap();

        let new_version = Version::from_str("2.0.0").unwrap();
        let updated = file.set_version(&new_version);

        assert!(updated.content.contains("version: 2.0.0"));
        assert!(updated.content.contains("other: stuff"));
    }

    #[test]
    fn test_readme_example() {
        let content = r"steps:
      - uses: knope-dev/action@v1
        with:
          version: 0.21.4";
        let regex = r"version:\s+(?<version>\d+\.\d+\.\d+)";

        let file = TextFile::new(
            RelativePathBuf::from("README.md"),
            content.to_string(),
            regex.to_string(),
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
        let regex = r"[invalid(regex";

        let result = TextFile::new(
            RelativePathBuf::from("test.txt"),
            content.to_string(),
            regex.to_string(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_pattern_no_match() {
        let content = "no version here";
        let regex = r"version:\s+(?<version>\d+\.\d+\.\d+)";

        let file = TextFile::new(
            RelativePathBuf::from("test.txt"),
            content.to_string(),
            regex.to_string(),
        )
        .unwrap();

        let result = file.get_version();
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_named_group() {
        let content = "version: 1.2.3";
        let regex = r"version:\s+(\d+\.\d+\.\d+)"; // No named group

        let result = TextFile::new(
            RelativePathBuf::from("test.txt"),
            content.to_string(),
            regex.to_string(),
        );

        assert!(result.is_err());
        if let Err(Error::MissingVersionGroup { .. }) = result {
            // Expected error
        } else {
            panic!("Expected MissingVersionGroup error");
        }
    }
}
