use std::path::PathBuf;

use miette::Diagnostic;
use platform_dirs::AppDirs;

use crate::{prompt, prompt::get_input};

/// For managing configuration of knope globally

pub(crate) fn get_or_prompt_for_email() -> Result<String, Error> {
    load_value_or_prompt("email", "Input your email address")
}

pub(crate) fn get_or_prompt_for_jira_token() -> Result<String, Error> {
    load_value_or_prompt("jira_token", "No Jira token found, generate one from https://id.atlassian.com/manage-profile/security/api-tokens and input here")
}

pub(crate) fn get_or_prompt_for_github_token() -> Result<String, Error> {
    std::env::var("GITHUB_TOKEN").or_else(|_| {
        load_value_or_prompt(
            "github_token",
            "No GitHub token found, generate one from https://github.com/settings/tokens with `repo` permissions and input here",
        )
    })
}

pub(crate) fn load_value_or_prompt(key: &str, prompt: &str) -> Result<String, Error> {
    let app_dirs = AppDirs::new(Some("knope"), true).ok_or(Error::CouldNotOpenConfigPath)?;
    let config_path = app_dirs.config_dir.join(key);
    if !app_dirs.config_dir.exists() {
        std::fs::create_dir_all(&app_dirs.config_dir)
            .map_err(|err| Error::CouldNotCreateDirectory(app_dirs.config_dir, err))?;
    }
    std::fs::read_to_string(&config_path).or_else(|_| {
        let contents = get_input(prompt)?;
        std::fs::write(config_path, &contents).map_err(Error::CouldNotWriteConfig)?;
        Ok(contents)
    })
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("Could not open configuration path")]
    #[diagnostic(
        code(app_config::could_not_open_config_path),
        help(
            "Knope attempts to store config in a local config directory, this error may be a \
                permissions issue or may mean you're using an obscure operating system"
        )
    )]
    CouldNotOpenConfigPath,
    #[error("Could not write config: {0}")]
    #[diagnostic(
        code(app_config::could_not_write_config),
        help(
            "Knope attempts to store config in a local config directory, this error may be a \
                    permissions issue or may mean you're using an obscure operating system"
        )
    )]
    CouldNotWriteConfig(std::io::Error),
    #[error("Could not create directory {0}: {1}")]
    #[diagnostic(
        code(app_config::could_not_create_directory),
        help("Failed to create the configuration directory, this is likely a permissions error.")
    )]
    CouldNotCreateDirectory(PathBuf, std::io::Error),
    #[error(transparent)]
    Prompt(#[from] prompt::Error),
}
