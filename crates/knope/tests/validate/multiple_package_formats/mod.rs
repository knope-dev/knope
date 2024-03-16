use crate::helpers::TestCase;

/// Run `--validate` with a config file that has both package configs—which is a conflict.
#[test]
fn validate_conflicting_packages() {
    TestCase::new(file!()).run("--validate");
}
