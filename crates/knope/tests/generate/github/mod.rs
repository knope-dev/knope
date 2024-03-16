use crate::helpers::*;

#[test]
fn https_remote() {
    TestCase::new(file!())
        .with_remote("https://github.com/knope-dev/knope.git")
        .run("--generate");
}

#[test]
fn ssh_remote() {
    TestCase::new(file!())
        .with_remote("git@github.com:knope-dev/knope.git")
        .run("--generate");
}
