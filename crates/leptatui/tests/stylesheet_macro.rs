//! Compile tests for the Leptatui stylesheet macro.

/// Verifies stylesheet macro pass and fail fixtures compile as expected.
///
/// # Example Under Test
///
/// ```text
/// tests/fixtures/stylesheet_macro/pass/*.rs
/// tests/fixtures/stylesheet_macro/fail/*.rs
/// ```
///
/// # Assertions
///
/// - Every pass fixture compiles successfully.
/// - Every fail fixture emits the expected compile error.
///
/// # Why
///
/// Stylesheet macro parsing should accept supported selectors and declarations
/// while rejecting unsupported syntax with stable diagnostics.
#[test]
fn stylesheet_macro_compile_cases() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/fixtures/stylesheet_macro/pass/*.rs");
    cases.compile_fail("tests/fixtures/stylesheet_macro/fail/*.rs");
}
