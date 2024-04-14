use crate::helpers::TestCase;

#[test]
fn not_allowed() {
    TestCase::new(file!()).run("--validate");
}
