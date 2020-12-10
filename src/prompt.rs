use std::fmt::Display;

use color_eyre::eyre::{eyre, Result, WrapErr};
use console::Term;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Select};

// TODO: Switch this to use references instead of owned types to avoid copies

pub(crate) fn select<T: Display>(mut items: Vec<T>, prompt: &str) -> Result<T> {
    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&items)
        .default(0)
        .with_prompt(prompt)
        .interact_on_opt(&Term::stdout())?;

    match selection {
        Some(index) => {
            let item = items.remove(index);
            Ok(item)
        }
        None => Err(eyre!("No option selected")),
    }
}

pub(crate) fn get_input(prompt: &str) -> Result<String> {
    Input::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .interact_text()
        .wrap_err("Failed to get input from user")
}
