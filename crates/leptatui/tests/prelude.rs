//! Prelude export tests.
//!
//! These tests ensure common runtime, node, style, context, and Leptos
//! reactivity APIs are available through [`leptatui::prelude`].

use leptatui::prelude::*;
use ratatui::{Terminal, backend::TestBackend};

#[component]
fn PreludeComponent() -> Node {
    provide_context(String::from("from prelude component"));
    let label = expect_context::<String>();

    view! { <Text>{label}</Text> }
}

#[test]
fn prelude_exposes_macros_and_required_context() -> Result<()> {
    let backend = TestBackend::new(32, 3);
    let mut terminal = Terminal::new(backend)?;
    let mut component = PreludeComponent::new();
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = Component::render(&mut component, &mut ctx);
    })?;
    render_result?;

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(rendered.contains("from prelude component"));

    Ok(())
}

/// Verifies the prelude exposes reactivity, context, nodes, and styles.
///
/// # Example Under Test
///
/// ```text
/// use leptatui::prelude::*;
/// signal(0)
/// provide_context(String::from("from prelude"))
/// block(column([text("from prelude"), button("OK")]))
/// ```
///
/// # Assertions
///
/// - Signals can be read, set, and updated from the prelude.
/// - A memo can derive from a prelude signal.
/// - Context values can be provided and read from the prelude.
/// - Node and style helpers type-check from the prelude.
#[test]
fn prelude_exposes_reactivity_and_context() {
    Owner::new().with(|| {
        let (count, set_count) = signal(0);

        assert_eq!(count.get_untracked(), 0);

        set_count.set(1);
        set_count.update(|value| *value += 1);

        assert_eq!(count.get_untracked(), 2);

        let doubled = Memo::new(move |_| count.get() * 2);

        assert_eq!(doubled.get_untracked(), 4);

        leptatui::context::__with_context_scope(|| {
            provide_context(String::from("from prelude"));

            assert_eq!(use_context::<String>().as_deref(), Some("from prelude"));
            assert_eq!(expect_context::<String>(), "from prelude");
        });

        let node: Node = block(column([text("from prelude"), button("OK")]));
        let _ = node;

        let style = TuiStyle::new()
            .foreground(Color::LightCyan)
            .background(Color::Black)
            .modifier(Modifier::BOLD)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .padding(TuiSpacing::uniform(1));
        let _ = style.to_block();
    });
}
