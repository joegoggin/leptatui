//! Compile tests for Leptatui component macros.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use leptatui::{AppControl, Component, RenderCtx, Result, button, component};
use ratatui::{Terminal, backend::TestBackend};

static MACRO_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
static MACRO_CONTEXT_OBSERVED: Mutex<Option<MacroLabel>> = Mutex::new(None);

/// Context value used by generated component provider tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MacroLabel(&'static str);

/// Component with an interactive button used by macro runtime tests.
#[component]
fn MacroButtonRoot() -> leptatui::Node {
    button("Save").on_press(|| {
        MACRO_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
        AppControl::Continue
    })
}

/// Component that records the label visible from its render context.
struct MacroContextConsumer;

impl Component for MacroContextConsumer {
    /// Records the context label visible during render.
    fn render(&mut self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        *MACRO_CONTEXT_OBSERVED
            .lock()
            .expect("context observation lock should be available") =
            leptatui::context::use_context::<MacroLabel>();
        Ok(())
    }
}

/// Component that provides context to a descendant component boundary.
#[component]
fn MacroContextProvider() -> leptatui::Node {
    leptatui::context::provide_context(MacroLabel("macro"));
    component(MacroContextConsumer)
}

/// Creates a key-press event for a key code.
fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

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

/// Verifies generated components dispatch events through their rendered tree.
///
/// # Example Under Test
///
/// ```text
/// #[component]
/// fn MacroButtonRoot() -> Node { button("Save").on_press(...) }
/// render, Tab, render, Enter
/// ```
///
/// # Assertions
///
/// - The first render initializes the generated component's node tree.
/// - Tab focuses the generated button node.
/// - A redraw after Tab preserves the focused node tree.
/// - Enter activates the focused button action.
///
/// # Why
///
/// App loops redraw between key events, so generated components must keep their
/// event-capable node tree across render passes.
#[test]
fn generated_components_dispatch_events_after_redraw() -> Result<()> {
    MACRO_BUTTON_PRESSES.store(0, Ordering::SeqCst);

    let backend = TestBackend::new(16, 3);
    let mut terminal = Terminal::new(backend)?;
    let mut component = MacroButtonRoot::new();
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = Component::render(&mut component, &mut ctx);
    })?;
    render_result?;

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Tab))?,
        AppControl::Continue
    );

    render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = Component::render(&mut component, &mut ctx);
    })?;
    render_result?;

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(MACRO_BUTTON_PRESSES.load(Ordering::SeqCst), 1);

    Ok(())
}

/// Verifies generated component providers remain visible to descendants.
///
/// # Example Under Test
///
/// ```text
/// #[component]
/// fn MacroContextProvider() -> Node {
///     provide_context(MacroLabel("macro"));
///     component(MacroContextConsumer)
/// }
/// ```
///
/// # Assertions
///
/// - The generated provider renders successfully.
/// - The descendant component reads the macro-provided context value.
///
/// # Why
///
/// Generated component bodies must provide context into the same render scope
/// used while rendering their returned node tree.
#[test]
fn generated_component_providers_are_visible_to_descendants() -> Result<()> {
    let backend = TestBackend::new(16, 3);
    let mut terminal = Terminal::new(backend)?;
    let mut component = MacroContextProvider::new();

    for _ in 0..2 {
        *MACRO_CONTEXT_OBSERVED
            .lock()
            .expect("context observation lock should be available") = None;

        let mut render_result = Ok(());
        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            render_result = Component::render(&mut component, &mut ctx);
        })?;
        render_result?;

        assert_eq!(
            *MACRO_CONTEXT_OBSERVED
                .lock()
                .expect("context observation lock should be available"),
            Some(MacroLabel("macro"))
        );
    }

    Ok(())
}

/// Verifies macros compile when the runtime crate dependency is renamed.
///
/// # Example Under Test
///
/// ```text
/// ui = { package = "leptatui", path = "..." }
/// use ui::prelude::*;
/// #[component]
/// fn Greeting() -> Node { view! { <Text>"hi"</Text> } }
/// ```
///
/// # Assertions
///
/// - `cargo check` succeeds in a temporary downstream crate.
/// - The downstream crate imports only the renamed `ui` dependency.
///
/// # Why
///
/// Generated proc-macro code should resolve the runtime crate path from the
/// caller's dependency name instead of hardcoding `::leptatui`.
#[test]
fn macros_compile_with_renamed_runtime_dependency() {
    let project_dir = create_alias_fixture();
    let output = Command::new("cargo")
        .arg("check")
        .arg("--quiet")
        .current_dir(&project_dir)
        .output()
        .expect("cargo check should run for alias fixture");

    assert!(
        output.status.success(),
        "cargo check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Creates a temporary crate that depends on Leptatui under a renamed package key.
///
/// # Returns
///
/// A [`PathBuf`] containing the generated fixture crate directory.
fn create_alias_fixture() -> PathBuf {
    let project_dir = alias_fixture_dir();
    if project_dir.exists() {
        fs::remove_dir_all(&project_dir).expect("stale alias fixture should be removable");
    }

    fs::create_dir_all(project_dir.join("src")).expect("alias fixture src should be creatable");

    let leptatui_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .expect("leptatui manifest directory should be canonicalizable");

    fs::write(
        project_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "leptatui-alias-check"
version = "0.0.0"
edition = "2024"

[dependencies]
ui = {{ package = "leptatui", path = "{}" }}
"#,
            leptatui_path.display()
        ),
    )
    .expect("alias fixture manifest should be writable");

    fs::write(
        project_dir.join("src/main.rs"),
        r#"use ui::prelude::*;

#[component]
fn Greeting() -> Node {
    view! { <Text>"hi"</Text> }
}

fn main() {
    let node: Node = Greeting::new().into();
    assert!(matches!(node, Node::Component(_)));
}
"#,
    )
    .expect("alias fixture source should be writable");

    project_dir
}

/// Returns a unique temporary directory for a renamed-dependency fixture crate.
///
/// # Returns
///
/// A [`PathBuf`] under Cargo's target temp directory or the system temp
/// directory.
fn alias_fixture_dir() -> PathBuf {
    let base = std::env::var_os("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after the Unix epoch")
        .as_nanos();

    base.join(format!(
        "leptatui-alias-check-{}-{timestamp}",
        std::process::id()
    ))
}
