use std::fmt::Display;

use console::Term;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use miette::Result;

use crate::step::StepError;

pub(crate) fn select<T: Display>(mut items: Vec<T>, prompt: &str) -> Result<T, StepError> {
    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&items)
        .default(0)
        .with_prompt(prompt)
        .interact_on_opt(&Term::stdout())
        .map_err(|e| StepError::UserInput(Some(e)))?;

    match selection {
        Some(index) => Ok(items.remove(index)),
        None => Err(StepError::UserInput(None)),
    }
}

pub(crate) fn get_input(prompt: &str) -> Result<String, StepError> {
    Input::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .interact_text()
        .map_err(|e| StepError::UserInput(Some(e)))
}
