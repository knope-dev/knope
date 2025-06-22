use crate::helpers::{GitCommand::Commit, TestCase};

#[test]
fn package_json_with_dependencies() {
    TestCase::new(file!())
        .git(&[
            Commit("feat(firstPackage)!: Breaking"),
            Commit("feat(secondPackage): Add new feature"),
            Commit("fix(thirdPackage): A fix"),
        ])
        .run("release");
}
