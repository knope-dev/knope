use crate::helpers::{GitCommand::*, TestCase};

#[test]
fn test() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("rust/v0.1.2"),
            Tag("python/v3.4.5"),
            Tag("javascript/v6.7.8"),
        ])
        .run("bump");
}
