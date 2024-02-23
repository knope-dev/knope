use crate::helpers::{GitCommand::*, TestCase};

/// Run `--validate` with a config file that has lots of problems.
#[test]
fn kitchen_sink() {
    TestCase::new(file!())
        .git([Commit("Initial commit"), Tag("1.0.0")])
        .run("--validate");
}
