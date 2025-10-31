use crate::helpers::{GitCommand::*, TestCase};

#[test]
fn text_file_minor() {
    TestCase::new(file!())
        .git(&[
            Commit("initial commit"),
            Tag("1.0.0"),
            Commit("feat: add feature"),
        ])
        .run("bump");
}
