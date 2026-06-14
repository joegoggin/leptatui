//! Prelude export tests.
//!
//! These tests ensure common runtime, view, style, context, and Leptos
//! reactivity APIs are available through [`leptatui::prelude`].

use leptatui::prelude::*;
use ratatui::{Terminal, backend::TestBackend};
use tokio::{task::yield_now, time::timeout};

use std::time::Duration;

/// Component used to prove prelude macro and context exports work together.
#[component]
fn PreludeComponent() -> View {
    provide_context(String::from("from prelude component"));
    let label = expect_context::<String>();
    let _example_key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE);

    use_key_event(KeyEventKind::Press, |key: KeyEvent| {
        if key.code == KeyCode::Char('p') {
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    view! { <Text>{label}</Text> }
}

/// Verifies prelude macro exports can render with required context.
///
/// # Example Under Test
///
/// ```text
/// #[component]
/// fn PreludeComponent() -> View {
///     provide_context(String::from("from prelude component"));
///     view! { <Text>{expect_context::<String>()}</Text> }
/// }
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The component render call succeeds.
/// - The rendered buffer contains `from prelude component`.
///
/// # Why
///
/// The prelude should expose enough macro, view, component, and context APIs for
/// a small component to render without extra imports.
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

/// Verifies the prelude exposes reactivity, context, views, and styles.
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
/// - View and style helpers type-check from the prelude.
/// - The stylesheet macro builds the expected stylesheet from prelude exports.
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

        let view: View = block(column([text("from prelude"), button("OK")]));
        let _ = view;

        let style = TuiStyle::new()
            .foreground(Color::LightCyan)
            .background(Color::Black)
            .modifier(Modifier::BOLD)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .padding(TuiSpacing::uniform(1));
        let _ = style.to_block();

        let stylesheet = stylesheet! {
            Text => { fg: Color::LightCyan }
        };
        assert_eq!(
            stylesheet,
            Stylesheet::new().rule(
                StyleSelector::view_type(ViewType::Text),
                TuiStyle::new().foreground(Color::LightCyan),
            )
        );
    });
}

/// Verifies the prelude exposes resource helpers.
#[tokio::test(flavor = "current_thread")]
async fn prelude_exposes_resource_helpers() {
    let owner = Owner::new();

    let resource: Resource<i32, &'static str> =
        owner.with(|| create_resource(|| (), |_| async { Ok(42) }));

    timeout(Duration::from_secs(1), async {
        loop {
            if matches!(resource.get_untracked(), ResourceState::Ready(42)) {
                break;
            }

            yield_now().await;
        }
    })
    .await
    .expect("resource should resolve from prelude exports");
}
