use crate::helpers::{
    GitCommand::{Commit, CommitWithAuthor, Tag},
    TestCase,
};

#[test]
fn notes() {
    TestCase::new(file!())
        .git(&[
            Commit("Existing version"),
            Tag("v1.0.0"),
            CommitWithAuthor {
                message: "feat: Feature from commit",
                name: "Sushi",
                email: "sushi@knope.tech",
            },
        ])
        .run("release");
}
