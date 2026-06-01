//! Compile tests for the Leptatui view macro.

#[test]
fn view_macro_compile_cases() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/fixtures/view_macro/pass/*.rs");
    cases.compile_fail("tests/fixtures/view_macro/fail/*.rs");
}
