use crate::helpers::TestCase;

#[test]
fn test() {
    TestCase::new(file!()).run("--generate");
}
