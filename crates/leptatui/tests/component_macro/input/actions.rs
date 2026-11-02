/// Records activation for the first wrapped button.
fn macro_first_wrapped_button_press() -> AppControl {
    MACRO_FIRST_WRAPPED_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
    AppControl::Continue
}

/// Records activation for the second wrapped button.
fn macro_second_wrapped_button_press() -> AppControl {
    MACRO_SECOND_WRAPPED_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
    AppControl::Continue
}

/// Records activation for the mixed built-in button.
fn macro_mixed_builtin_button_press() -> AppControl {
    MACRO_MIXED_BUILTIN_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
    AppControl::Continue
}

/// Records activation for the mixed wrapped button.
fn macro_mixed_wrapped_button_press() -> AppControl {
    MACRO_MIXED_WRAPPED_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
    AppControl::Continue
}

/// Records activation for a component-wrapped button rebuilt by a dynamic view.
///
/// # Returns
///
/// An [`AppControl`] value that keeps the test application running.
fn macro_dynamic_wrapped_button_press() -> AppControl {
    MACRO_DYNAMIC_WRAPPED_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
    AppControl::Continue
}
