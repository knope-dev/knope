use std::path::Path;

use miette::Diagnostic;
use thiserror::Error;

use crate::{
    fs,
    releases::{git, semver::Version},
};

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error("No module line found in go.mod file")]
    #[diagnostic(
        code(go::no_module_line),
        help("The go.mod file does not contain a module line. This is required for the step to work."),
    )]
    MissingModuleLine,
    #[error("The module line {0} in go.mod could not be parsed")]
    #[diagnostic(
        code(go::malformed_module_line),
        help("The go.mod file contains an invalid module line.")
    )]
    MalformedModuleLine(String),
    #[error(transparent)]
    Git(#[from] git::Error),
    #[error(transparent)]
    Fs(#[from] fs::Error),
}

pub(crate) fn set_version(
    dry_run: &mut Option<Box<dyn std::io::Write>>,
    content: String,
    new_version: &Version,
    path: &Path,
) -> Result<String, Error> {
    let parent_dir = path.parent().map(Path::to_string_lossy);
    if let Some(parent_dir) = parent_dir {
        if !parent_dir.is_empty() {
            let tag = format!("{parent_dir}/v{new_version}");
            git::create_tag(dry_run, tag)?;
        }
        // If there's not a nested dir, the tag will equal the release tag, so creating it here would cause a conflict later.
    }

    let new_major = new_version.stable_component().major;
    if new_major == 0 || new_major == 1 {
        // These major versions aren't recorded in go.mod
        return Ok(content);
    }

    let module_line = content
        .lines()
        .find(|line| line.starts_with("module "))
        .ok_or(Error::MissingModuleLine)?;
    let module = module_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| Error::MalformedModuleLine(String::from(module_line)))?;
    let (uri, last_part) = module
        .rsplit_once('/')
        .ok_or_else(|| Error::MalformedModuleLine(String::from(module_line)))?;
    let existing_version = if let Some(major_string) = last_part.strip_prefix('v') {
        if let Ok(major) = major_string.parse::<u64>() {
            Some(major)
        } else {
            None
        }
    } else {
        None
    };
    let new_contents = if let Some(existing_version) = existing_version {
        if existing_version == new_version.stable_component().major {
            None
        } else {
            let new_version_string = format!("v{new_major}");
            let new_module_line = format!("module {uri}/{new_version_string}");
            Some(content.replace(module_line, &new_module_line))
        }
    } else {
        // No existing version found—add new line
        let new_module_line = format!("module {module}/v{new_major}");
        Some(content.replace(module_line, &new_module_line))
    };
    if let Some(new_contents) = new_contents {
        fs::write(dry_run, &new_version.to_string(), path, &new_contents)?;
        Ok(new_contents)
    } else {
        Ok(content)
    }
}
