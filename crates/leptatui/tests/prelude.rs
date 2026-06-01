//! Prelude export tests.
//!
//! These tests ensure common runtime, node, style, context, and Leptos
//! reactivity APIs are available through [`leptatui::prelude`].

use leptatui::prelude::*;

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

        assert_eq!(count.get(), 0);

        set_count.set(1);
        set_count.update(|value| *value += 1);

        assert_eq!(count.get(), 2);

        let doubled = Memo::new(move |_| count.get() * 2);

        assert_eq!(doubled.get(), 4);

        provide_context(String::from("from prelude"));

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

        assert_eq!(use_context::<String>().as_deref(), Some("from prelude"));
    });
}
