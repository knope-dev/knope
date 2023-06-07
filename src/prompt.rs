use std::fmt::Display;

use inquire::{Password, Select};
use miette::Result;

use crate::step::StepError;

pub(crate) fn select<T: Display>(items: Vec<T>, prompt: &str) -> Result<T, StepError> {
    Select::new(prompt, items)
        .prompt()
        .map_err(StepError::UserInput)
}

pub(crate) fn get_input(prompt: &str) -> Result<String, StepError> {
    Password::new(prompt)
        .with_display_toggle_enabled()
        .without_confirmation()
        .prompt()
        .map_err(StepError::UserInput)
}
