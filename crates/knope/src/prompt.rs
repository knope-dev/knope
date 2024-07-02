use std::fmt::Display;

use inquire::{Confirm, InquireError, Password, Select};
use miette::{Diagnostic, Result};

pub(crate) fn select<T: Display>(items: Vec<T>, prompt: &str) -> Result<T, Error> {
    Select::new(prompt, items).prompt().map_err(Error)
}

pub(crate) fn get_input(prompt: &str) -> Result<String, Error> {
    Password::new(prompt)
        .with_display_toggle_enabled()
        .without_confirmation()
        .prompt()
        .map_err(Error)
}

pub(crate) fn confirm(prompt: &str) -> Result<bool, Error> {
    Confirm::new(prompt)
        .with_default(true)
        .prompt()
        .map_err(Error)
}

#[derive(Debug, Diagnostic, thiserror::Error)]
#[error("Failed to get user input")]
#[diagnostic(
    code(prompt),
    help("This step requires user input, but no user input was provided. Try running the step again."),
)]
pub(crate) struct Error(#[from] InquireError);
