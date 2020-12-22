use color_eyre::eyre::{Result, WrapErr};
use git_conventional::Commit;

use crate::git::get_commit_messages_after_last_tag;

fn get_conventional_commits_after_last_tag() -> Result<Vec<Commit>> {
    let commit_messages = get_commit_messages_after_last_tag()
        .wrap_err("Could not get commit messages after last tag.")?;
    Ok(commit_messages
        .into_iter()
        .filter_map(|message| Commit::parse(&message).ok())
        .collect())
}
