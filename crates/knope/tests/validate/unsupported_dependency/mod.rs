use crate::helpers::TestCase;

#[test]
fn validate_dependency_formats() {
    TestCase::new(file!()).run("--validate");
}
