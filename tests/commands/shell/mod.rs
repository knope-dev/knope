use crate::helpers::TestCase;

#[test]
fn shell() {
    TestCase::new(file!())
        .env("AN_ENV_VAR", "a value")
        .run("shell-command");
}
