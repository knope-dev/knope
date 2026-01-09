use snapbox::{Assert, Redactions};
use time::{OffsetDateTime, macros::format_description};

/// Assert that includes [DATE] and [COMMIT] substitutions
pub fn assert(normalize_paths: bool) -> Assert {
    let mut redactions = Redactions::default();
    let time_format = format_description!("[year]-[month]-[day]");
    redactions
        .insert(
            "[DATE]",
            OffsetDateTime::now_utc().format(time_format).unwrap(),
        )
        .unwrap();
    redactions
        .insert("[EXE]", std::env::consts::EXE_SUFFIX)
        .unwrap();
    // Git commit SHA (40 hex characters)
    redactions
        .insert("[COMMIT]", regex::Regex::new("[0-9a-f]{40}").unwrap())
        .unwrap();
    Assert::new()
        .redact_with(redactions)
        .action_env("SNAPSHOTS")
        .normalize_paths(normalize_paths)
}
