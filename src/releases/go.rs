use miette::Diagnostic;
use thiserror::Error;

use crate::releases::semver::Version;

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
}

pub(crate) fn set_version(go_mod: String, new_version: &Version) -> Result<String, Error> {
    let new_major = new_version.stable_component().major;
    if new_major == 0 || new_major == 1 {
        // These major versions aren't recorded in go.mod
        return Ok(go_mod);
    }

    let module_line = go_mod
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
    if let Some(existing_version) = existing_version {
        if existing_version == new_version.stable_component().major {
            // Major version has not changed—keep existing content
            return Ok(go_mod);
        }
        let new_version_string = format!("v{new_major}");
        let new_module_line = format!("module {uri}/{new_version_string}");
        Ok(go_mod.replace(module_line, &new_module_line))
    } else {
        // No existing version found—add new line
        let new_module_line = format!("module {module}/v{new_major}");
        Ok(go_mod.replace(module_line, &new_module_line))
    }
}
