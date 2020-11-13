use color_eyre::eyre::{eyre, Result};
use console::Term;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use std::fmt::Display;

// TODO: Switch this to use references instead of owned types to avoid copies

pub fn select<T: Display>(mut items: Vec<T>, prompt: &str) -> Result<T> {
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
