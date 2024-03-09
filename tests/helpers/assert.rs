use snapbox::{Assert, Substitutions};
use time::{macros::format_description, OffsetDateTime};

/// Assert that includes [DATE] substitution
pub fn assert(normalize_paths: bool) -> Assert {
    let mut substitutions = Substitutions::default();
    let time_format = format_description!("[year]-[month]-[day]");
    substitutions
        .insert(
            "[DATE]",
            OffsetDateTime::now_utc().format(time_format).unwrap(),
        )
        .unwrap();
    Assert::new()
        .substitutions(substitutions)
        .action_env("SNAPSHOTS")
        .normalize_paths(normalize_paths)
}
