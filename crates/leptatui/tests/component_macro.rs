//! Compile tests for Leptatui component macros.

#[test]
fn component_macro_compile_cases() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/fixtures/component_macro/pass/*.rs");
    cases.compile_fail("tests/fixtures/component_macro/fail/*.rs");
}
