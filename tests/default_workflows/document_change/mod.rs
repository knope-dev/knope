use crate::helpers::TestCase;

#[test]
fn help() {
    TestCase::new(file!()).run("document-change --help");
}
