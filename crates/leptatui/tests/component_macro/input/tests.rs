/// Verifies generated components dispatch key events through registered hooks.
///
/// # Example Under Test
///
/// ```text
/// #[component]
/// fn MacroButtonRoot() -> impl IntoView { use_key_event(Press, ...); button("Save").on_press(...) }
/// render, Repeat(s), Press(s)
/// ```
///
/// # Assertions
///
/// - The first render initializes the generated component's view tree.
/// - A repeat event does not invoke the press-only handler.
/// - A press event invokes the component key handler.
/// - Unhandled keys continue without invoking the handler.
///
/// # Why
///
/// Generated components should support custom key maps without requiring a
/// manual [`View`] implementation.
#[test]
fn generated_components_dispatch_registered_key_handlers() -> leptatui::app::Result<()> {
    MACRO_BUTTON_PRESSES.store(0, Ordering::SeqCst);

    let backend = TestBackend::new(16, 3);
    let mut terminal = Terminal::new(backend)?;
    let mut component = MacroButtonRoot::new();
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = View::render(&component, &mut ctx);
    })?;
    render_result?;

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('x')))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_BUTTON_PRESSES.load(Ordering::SeqCst), 0);

    assert_eq!(
        View::handle_event(
            &mut component,
            key_with_kind(KeyCode::Char('s'), KeyEventKind::Repeat),
        )?,
        AppControl::Continue
    );
    assert_eq!(MACRO_BUTTON_PRESSES.load(Ordering::SeqCst), 0);

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('s')))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_BUTTON_PRESSES.load(Ordering::SeqCst), 1);

    Ok(())
}

/// Verifies key-event hooks can target repeat and release events explicitly.
///
/// # Example Under Test
///
/// ```text
/// use_key_event(Repeat, ...)
/// use_key_event(Release, ...)
/// Press(k), Repeat(k), Release(k)
/// ```
///
/// # Assertions
///
/// - The repeat handler ignores press and release events.
/// - The release handler ignores press and repeat events.
#[test]
fn generated_key_event_handlers_filter_by_event_kind() -> leptatui::app::Result<()> {
    MACRO_REPEAT_KEY_PRESSES.store(0, Ordering::SeqCst);
    MACRO_RELEASE_KEY_PRESSES.store(0, Ordering::SeqCst);

    let mut component = MacroKindSpecificKeyHandlers::new();

    assert_eq!(
        View::handle_event(
            &mut component,
            key_with_kind(KeyCode::Char('k'), KeyEventKind::Press),
        )?,
        AppControl::Continue
    );
    assert_eq!(MACRO_REPEAT_KEY_PRESSES.load(Ordering::SeqCst), 0);
    assert_eq!(MACRO_RELEASE_KEY_PRESSES.load(Ordering::SeqCst), 0);

    assert_eq!(
        View::handle_event(
            &mut component,
            key_with_kind(KeyCode::Char('k'), KeyEventKind::Repeat),
        )?,
        AppControl::Continue
    );
    assert_eq!(MACRO_REPEAT_KEY_PRESSES.load(Ordering::SeqCst), 1);
    assert_eq!(MACRO_RELEASE_KEY_PRESSES.load(Ordering::SeqCst), 0);

    assert_eq!(
        View::handle_event(
            &mut component,
            key_with_kind(KeyCode::Char('k'), KeyEventKind::Release),
        )?,
        AppControl::Continue
    );
    assert_eq!(MACRO_REPEAT_KEY_PRESSES.load(Ordering::SeqCst), 1);
    assert_eq!(MACRO_RELEASE_KEY_PRESSES.load(Ordering::SeqCst), 1);

    Ok(())
}

/// Verifies generated components use default button keys when hooks pass.
///
/// # Example Under Test
///
/// ```text
/// #[component]
/// fn MacroDefaultButtonRoot() -> impl IntoView { button("Default").on_press(...) }
/// Tab, Enter
/// ```
///
/// # Assertions
///
/// - Tab focuses the generated button view.
/// - Enter activates the focused button.
#[test]
fn generated_components_run_default_button_keys_after_hook_pass() -> leptatui::app::Result<()> {
    MACRO_DEFAULT_BUTTON_PRESSES.store(0, Ordering::SeqCst);

    let mut component = MacroDefaultButtonRoot::new();

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Tab))?,
        AppControl::Continue
    );
    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_DEFAULT_BUTTON_PRESSES.load(Ordering::SeqCst), 1);

    Ok(())
}

/// Verifies default focus traversal crosses generated component boundaries.
///
/// # Example Under Test
///
/// ```text
/// Row(<WrappedButton first />, <WrappedButton second />)
/// Tab, Enter, Tab, Enter, BackTab, Enter
/// ```
///
/// # Assertions
///
/// - The first Tab focuses the first wrapped button.
/// - The second Tab focuses the second wrapped button.
/// - BackTab returns focus to the first wrapped button.
#[test]
fn generated_component_focus_crosses_sibling_component_boundaries() -> leptatui::app::Result<()> {
    MACRO_FIRST_WRAPPED_BUTTON_PRESSES.store(0, Ordering::SeqCst);
    MACRO_SECOND_WRAPPED_BUTTON_PRESSES.store(0, Ordering::SeqCst);

    let mut component = MacroWrappedButtonSiblings::new();

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Tab))?,
        AppControl::Continue
    );
    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_FIRST_WRAPPED_BUTTON_PRESSES.load(Ordering::SeqCst), 1);
    assert_eq!(
        MACRO_SECOND_WRAPPED_BUTTON_PRESSES.load(Ordering::SeqCst),
        0
    );

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Tab))?,
        AppControl::Continue
    );
    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_FIRST_WRAPPED_BUTTON_PRESSES.load(Ordering::SeqCst), 1);
    assert_eq!(
        MACRO_SECOND_WRAPPED_BUTTON_PRESSES.load(Ordering::SeqCst),
        1
    );

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::BackTab))?,
        AppControl::Continue
    );
    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_FIRST_WRAPPED_BUTTON_PRESSES.load(Ordering::SeqCst), 2);
    assert_eq!(
        MACRO_SECOND_WRAPPED_BUTTON_PRESSES.load(Ordering::SeqCst),
        1
    );

    Ok(())
}

/// Verifies built-in buttons and component-wrapped buttons share focus order.
///
/// # Example Under Test
///
/// ```text
/// MacroMixedButtonSiblings(BuiltIn, MacroWrappedButton)
/// Tab, Enter, Tab, Enter
/// ```
///
/// # Assertions
///
/// - The first activation invokes only the built-in button callback.
/// - The second activation invokes only the wrapped button callback.
/// - Focus and activation events continue through the component root.
#[test]
fn generated_component_focus_mixes_static_and_component_buttons() -> leptatui::app::Result<()> {
    MACRO_MIXED_BUILTIN_BUTTON_PRESSES.store(0, Ordering::SeqCst);
    MACRO_MIXED_WRAPPED_BUTTON_PRESSES.store(0, Ordering::SeqCst);

    let mut component = MacroMixedButtonSiblings::new();

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Tab))?,
        AppControl::Continue
    );
    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_MIXED_BUILTIN_BUTTON_PRESSES.load(Ordering::SeqCst), 1);
    assert_eq!(MACRO_MIXED_WRAPPED_BUTTON_PRESSES.load(Ordering::SeqCst), 0);

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Tab))?,
        AppControl::Continue
    );
    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_MIXED_BUILTIN_BUTTON_PRESSES.load(Ordering::SeqCst), 1);
    assert_eq!(MACRO_MIXED_WRAPPED_BUTTON_PRESSES.load(Ordering::SeqCst), 1);

    Ok(())
}

/// Verifies redraws preserve focus inside rebuilt prop-bearing components.
///
/// # Example Under Test
///
/// ```text
/// dynamic(|| <MacroWrappedButton label=signal_value />)
/// render, Tab, update signal, render, Enter
/// ```
///
/// # Assertions
///
/// - The redraw renders the component's current prop value.
/// - The compatible nested button retains focus across the redraw.
/// - Enter activates the focused button after reconciliation.
///
/// # Why
///
/// Replacing a prop-bearing component must not discard descendant focus between
/// a focus event and the following redraw.
#[test]
fn generated_dynamic_component_redraw_preserves_nested_button_focus()
-> leptatui::app::Result<()> {
    MACRO_DYNAMIC_WRAPPED_BUTTON_PRESSES.store(0, Ordering::SeqCst);

    let mut component = MacroDynamicWrappedButtonRoot::new();
    let terminal = render_component(&mut component, 16, 3)?;
    assert!(rendered_text(&terminal).contains("Before"));

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Tab))?,
        AppControl::Continue
    );
    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('u')))?,
        AppControl::Continue
    );

    let terminal = render_component(&mut component, 16, 3)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("After"), "rendered text: {text:?}");
    assert!(!text.contains("Before"), "rendered text: {text:?}");

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_DYNAMIC_WRAPPED_BUTTON_PRESSES.load(Ordering::SeqCst), 1);

    Ok(())
}

/// Verifies local hooks can override default Tab focus movement.
///
/// # Example Under Test
///
/// ```text
/// use_key_event(Press, Tab => Handled)
/// button("Default").on_press(...)
/// Tab, Enter
/// ```
///
/// # Assertions
///
/// - Tab is consumed by the hook.
/// - Enter does not activate the button because focus did not move.
#[test]
fn generated_component_hook_can_override_default_tab_focus() -> leptatui::app::Result<()> {
    MACRO_DEFAULT_BUTTON_PRESSES.store(0, Ordering::SeqCst);

    let mut component = MacroTabOverrideButtonRoot::new();

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Tab))?,
        AppControl::Continue
    );
    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_DEFAULT_BUTTON_PRESSES.load(Ordering::SeqCst), 0);

    Ok(())
}

/// Verifies local hooks can override default Enter activation.
///
/// # Example Under Test
///
/// ```text
/// use_key_event(Press, Enter => Handled)
/// button("Default").on_press(...)
/// Tab, Enter
/// ```
///
/// # Assertions
///
/// - Tab uses the default focus behavior.
/// - Enter is consumed by the hook before the focused button activates.
#[test]
fn generated_component_hook_can_override_default_enter_activation() -> leptatui::app::Result<()> {
    MACRO_DEFAULT_BUTTON_PRESSES.store(0, Ordering::SeqCst);

    let mut component = MacroEnterOverrideButtonRoot::new();

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Tab))?,
        AppControl::Continue
    );
    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_DEFAULT_BUTTON_PRESSES.load(Ordering::SeqCst), 0);

    Ok(())
}
