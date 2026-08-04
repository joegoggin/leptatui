//! Pass fixture for fallible generated components.
//!
//! This binary verifies `ViewResult`, explicit result errors, `?`, and both
//! `view_error!` forms compile through generated component setup.

use leptatui::prelude::*;

/// Loads a value through a standard error type.
///
/// # Returns
///
/// A string value when the synthetic load succeeds.
///
/// # Errors
///
/// Returns [`std::io::Error`] when the synthetic load fails.
fn load_value() -> std::io::Result<String> {
    Ok(String::from("loaded"))
}

/// Builds a component that propagates a standard error.
///
/// # Returns
///
/// A text view containing the loaded value.
///
/// # Errors
///
/// Returns [`ViewError`] when loading fails.
#[component]
fn Propagated() -> ViewResult<impl IntoView> {
    let value = load_value()?;
    view! { <Text>{value}</Text> }
}

/// Builds a component that replaces an error with a formatted message.
///
/// # Returns
///
/// A loaded-state text view.
///
/// # Errors
///
/// Returns [`ViewError`] with a formatted replacement message.
#[component]
fn Replaced() -> ViewResult<impl IntoView> {
    let path = "fixture.txt";
    if load_value().is_err() {
        view_error!("could not load {path}");
    }
    view! { <Text>"loaded"</Text> }
}

/// Builds a component that preserves its source error as context.
///
/// # Returns
///
/// A loaded-state text view.
///
/// # Errors
///
/// Returns [`ViewError`] retaining its source error beneath custom context.
#[component]
fn Contextual() -> ViewResult<impl IntoView> {
    if let Err(error) = load_value() {
        view_error!(error => "could not load fixture {}", 1);
    }
    view! { <Text>"loaded"</Text> }
}

/// Builds a component with an explicit concrete error type.
///
/// # Returns
///
/// A text view containing the loaded value.
///
/// # Errors
///
/// Returns [`std::io::Error`] when loading fails.
#[component]
fn Explicit() -> std::result::Result<impl IntoView, std::io::Error> {
    let value = load_value()?;
    view! { <Text>{value}</Text> }
}

/// Constructs every fallible fixture component.
///
/// # Returns
///
/// The unit value after every component is constructed.
fn main() {
    let _ = Propagated::new();
    let _ = Replaced::new();
    let _ = Contextual::new();
    let _ = Explicit::new();
}
