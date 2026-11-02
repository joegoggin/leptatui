/// Verifies generated component key handlers can exit the app.
///
/// # Example Under Test
///
/// ```text
/// use_key_event(Press, |key| if key == q { KeyControl::Exit } else { KeyControl::Pass })
/// ```
///
/// # Assertions
///
/// - Unhandled keys continue the app.
/// - The `q` key returns [`AppControl::Exit`].
#[test]
fn generated_component_key_handler_can_exit() -> leptatui::app::Result<()> {
    let mut component = MacroKeyExitRoot::new();

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('x')))?,
        AppControl::Continue
    );
    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('q')))?,
        AppControl::Exit
    );

    Ok(())
}

/// Verifies child key handlers override parent key handlers.
///
/// # Example Under Test
///
/// ```text
/// parent use_key_event(Press, x => Handled)
/// child use_key_event(Press, x => Handled)
/// ```
///
/// # Assertions
///
/// - The child handler observes `x`.
/// - The parent handler does not observe `x`.
#[test]
fn child_key_handler_overrides_parent_handler() -> leptatui::app::Result<()> {
    MACRO_PARENT_KEY_PRESSES.store(0, Ordering::SeqCst);
    MACRO_CHILD_KEY_PRESSES.store(0, Ordering::SeqCst);

    let mut component = MacroParentKeyRoot::new();

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('x')))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_CHILD_KEY_PRESSES.load(Ordering::SeqCst), 1);
    assert_eq!(MACRO_PARENT_KEY_PRESSES.load(Ordering::SeqCst), 0);

    Ok(())
}

/// Verifies child pass-through lets parent handlers run.
///
/// # Example Under Test
///
/// ```text
/// parent use_key_event(Press, p => Handled)
/// child use_key_event(Press, p => Pass)
/// ```
///
/// # Assertions
///
/// - The child handler observes `p`.
/// - The parent handler handles `p`.
#[test]
fn child_key_pass_reaches_parent_handler() -> leptatui::app::Result<()> {
    MACRO_PASS_PARENT_KEY_PRESSES.store(0, Ordering::SeqCst);
    MACRO_PASS_CHILD_KEY_PRESSES.store(0, Ordering::SeqCst);

    let mut component = MacroParentAfterPassRoot::new();

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('p')))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_PASS_CHILD_KEY_PRESSES.load(Ordering::SeqCst), 1);
    assert_eq!(MACRO_PASS_PARENT_KEY_PRESSES.load(Ordering::SeqCst), 1);

    Ok(())
}

/// Verifies handlers in one component run in registration order.
///
/// # Example Under Test
///
/// ```text
/// use_key_event(Press, m => Pass)
/// use_key_event(Press, m => Handled)
/// use_key_event(Press, m => Handled)
/// ```
///
/// # Assertions
///
/// - The first passing handler runs.
/// - The second handling callback runs.
/// - The third callback does not run.
#[test]
fn component_key_handlers_short_circuit_in_registration_order() -> leptatui::app::Result<()> {
    MACRO_FIRST_KEY_HANDLER.store(0, Ordering::SeqCst);
    MACRO_SECOND_KEY_HANDLER.store(0, Ordering::SeqCst);
    MACRO_THIRD_KEY_HANDLER.store(0, Ordering::SeqCst);

    let mut component = MacroMultipleKeyHandlers::new();

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('m')))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_FIRST_KEY_HANDLER.load(Ordering::SeqCst), 1);
    assert_eq!(MACRO_SECOND_KEY_HANDLER.load(Ordering::SeqCst), 1);
    assert_eq!(MACRO_THIRD_KEY_HANDLER.load(Ordering::SeqCst), 0);

    Ok(())
}
