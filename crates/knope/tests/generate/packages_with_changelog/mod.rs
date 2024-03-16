use crate::helpers::TestCase;

#[test]
fn generate() {
    TestCase::new(file!()).run("--generate");
}
