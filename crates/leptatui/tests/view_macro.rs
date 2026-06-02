//! Compile tests for the Leptatui view macro.

/// Verifies view macro pass and fail fixtures compile as expected.
///
/// # Example Under Test
///
/// ```text
/// tests/fixtures/view_macro/pass/*.rs
/// tests/fixtures/view_macro/fail/*.rs
/// ```
///
/// # Assertions
///
/// - Every pass fixture compiles successfully.
/// - Every fail fixture emits the expected compile error.
///
/// # Why
///
/// View syntax should expand supported terminal elements and reject unsupported
/// elements or child shapes with stable diagnostics.
#[test]
fn view_macro_compile_cases() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/fixtures/view_macro/pass/*.rs");
    cases.compile_fail("tests/fixtures/view_macro/fail/*.rs");
}
