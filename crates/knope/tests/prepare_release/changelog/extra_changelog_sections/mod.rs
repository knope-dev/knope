use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn notes() {
    TestCase::new(file!())
        .git(&[
            Commit("Existing versions"),
            Tag("first/v1.0.0"),
            Tag("second/v0.1.0"),
            Commit("chore: something\n\nChangelog-Note: A standard note"),
            Commit("chore(first): something\n\nChangelog-Note: Standard note first only"),
            Commit("chore(second): something\n\nChangelog-Note: Standard note second only"),
            Commit("chore: something\n\nChangelog-First-Note: A custom note"),
            Commit("chore: something\n\nSpecial: Special note"),
            Commit("chore: something\n\nWhatever: Whatever note"),
        ])
        .run("release");
}
