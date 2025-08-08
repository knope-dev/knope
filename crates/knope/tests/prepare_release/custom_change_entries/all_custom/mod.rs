use crate::helpers::{
    GitCommand::{Add, Commit, CommitWithAuthor, Tag},
    TestCase,
};

#[test]
fn notes() {
    TestCase::new(file!())
        .git(&[
            Commit("Existing version"),
            Tag("v1.0.0"),
            Add(".changeset/breaking_change.md"),
            CommitWithAuthor {
                message: "Add breaking change file",
                name: "Alice",
                email: "alice@knope.tech",
            },
            CommitWithAuthor {
                message: "feat: Feature from commit",
                name: "Sushi",
                email: "sushi@knope.tech",
            },
        ])
        .run("release");
}
