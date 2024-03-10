use crate::helpers::{GitCommand::*, *};

#[test]
fn https_remote() {
    TestCase::new(file!())
        .with_remote("https://codeberg.org/knope-dev/knope.git")
        .run("--generate");
}

#[test]
fn https_without_git_suffix() {
    TestCase::new(file!())
        .with_remote("https://codeberg.org/knope-dev/knope")
        .run("--generate");
}

#[test]
fn ssh_remote() {
    TestCase::new(file!())
        .with_remote("git@codeberg.org:knope-dev/knope.git")
        .run("--generate");
}

#[test]
fn ssh_without_git_suffix() {
    TestCase::new(file!())
        .with_remote("git@codeberg.org:knope-dev/knope")
        .run("--generate");
}
