use platform_dirs::AppDirs;

use crate::prompt::get_input;
use crate::step::StepError;

/// For managing configuration of Dobby globally

pub(crate) fn get_or_prompt_for_email() -> Result<String, StepError> {
    load_value_or_prompt("email", "Input your email address")
}

pub(crate) fn get_or_prompt_for_jira_token() -> Result<String, StepError> {
    load_value_or_prompt("jira_token", "No Jira token found, generate one from https://id.atlassian.com/manage-profile/security/api-tokens and input here")
}

pub(crate) fn get_or_prompt_for_github_token() -> Result<String, StepError> {
    load_value_or_prompt(
        "github_token", 
        "No GitHub token found, generate one from https://github.com/settings/tokens with `repo` permissions and input here"
    )
}

pub(crate) fn load_value_or_prompt(key: &str, prompt: &str) -> Result<String, StepError> {
    let app_dirs = AppDirs::new(Some("dobby"), true).expect("Could not open config path");
    let config_path = app_dirs.config_dir.join(key);
    if !app_dirs.config_dir.exists() {
        std::fs::create_dir_all(app_dirs.config_dir)?;
    }
    std::fs::read_to_string(&config_path).or_else(|_| {
        let contents = get_input(prompt)?;
        std::fs::write(config_path, &contents)?;
        Ok(contents)
    })
}
