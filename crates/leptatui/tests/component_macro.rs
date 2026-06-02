//! Compile tests for Leptatui component macros.

/// Verifies component macro pass and fail fixtures compile as expected.
///
/// # Example Under Test
///
/// ```text
/// tests/fixtures/component_macro/pass/*.rs
/// tests/fixtures/component_macro/fail/*.rs
/// ```
///
/// # Assertions
///
/// - Every pass fixture compiles successfully.
/// - Every fail fixture emits the expected compile error.
///
/// # Why
///
/// Component macro validation should reject unsupported signatures while
/// preserving accepted component conversions.
#[test]
fn component_macro_compile_cases() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/fixtures/component_macro/pass/*.rs");
    cases.compile_fail("tests/fixtures/component_macro/fail/*.rs");
}
