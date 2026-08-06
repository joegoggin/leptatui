//! Compile tests for typed route and query parameter derives.

/// Verifies typed parameter derive fixtures compile or fail as expected.
///
/// # Example Under Test
///
/// ```text
/// tests/fixtures/route_params_macro/pass/*.rs
/// tests/fixtures/route_params_macro/fail/*.rs
/// ```
///
/// # Assertions
///
/// - Every supported named, non-generic struct fixture compiles successfully.
/// - Unsupported derive targets and mappings emit their expected diagnostics.
#[test]
fn route_params_macro_compile_cases() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/fixtures/route_params_macro/pass/*.rs");
    cases.compile_fail("tests/fixtures/route_params_macro/fail/*.rs");
}
