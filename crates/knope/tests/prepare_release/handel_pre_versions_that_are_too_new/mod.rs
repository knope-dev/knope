use crate::helpers::{
    GitCommand::{Commit, Tag},
    TestCase,
};

#[test]
fn handle_pre_versions_that_are_too_new() {
    TestCase::new(file!())
        .git(&[
            Commit("Initial commit"),
            Tag("v2.0.0-rc.0"), // An earlier pre-release, but 2.0 isn't finished yet!
            Tag("v1.2.3"),      // The current stable version
            Commit("feat: A new feature"),
            Tag("v1.3.0-rc.0"), // The current pre-release version
            Commit("feat: Another new feature"),
        ])
        .run("prerelease");
}
