//! Prelude export tests.
//!
//! These tests ensure common runtime, view, style, context, and Leptos
//! reactivity APIs are available through [`leptatui::prelude`].

use leptatui::prelude::*;
use ratatui::{Terminal, backend::TestBackend};

mod support;

use support::wait_until;

/// Renders a label from context using only prelude exports.
///
/// # Returns
///
/// A [`View`] containing the context label.
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
/// block(column([
///     form([input("Ada"), text_area("Notes")]),
///     image("logo.png"),
///     progress_bar(0.5),
/// ]))
/// ```
///
/// # Assertions
///
/// - Signals can be read, set, and updated from the prelude.
/// - A memo can derive from a prelude signal.
/// - Context values can be provided and read from the prelude.
/// - New standard-library builders, callback aliases, and source types
///   type-check from the prelude.
/// - The view and stylesheet macros support the new standard-library
///   component names from prelude imports.
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

        leptatui::__private::__with_context_scope(|| {
            provide_context(String::from("from prelude"));

            assert_eq!(use_context::<String>().as_deref(), Some("from prelude"));
            assert_eq!(expect_context::<String>(), "from prelude");
        });

        let form_action: FormAction = std::rc::Rc::new(|| AppControl::Continue);
        let input_action: InputAction = std::rc::Rc::new(|_| AppControl::Continue);
        let input_action_for_input = std::rc::Rc::clone(&input_action);
        let input_action_for_text_area = std::rc::Rc::clone(&input_action);
        let form_action_for_submit = std::rc::Rc::clone(&form_action);
        let image_source = ImageSource::from("assets/logo.png");
        assert_eq!(image_source, ImageSource::Path("assets/logo.png".into()));

        let input_view = input("Ada").on_input(move |next| input_action_for_input(next));
        let text_area_view =
            text_area("Notes").on_input(move |next| input_action_for_text_area(next));
        let form_view = form([input_view, text_area_view])
            .on_submit(move || form_action_for_submit())
            .on_cancel(|| AppControl::Continue);
        let view: View = block(column([
            text("from prelude"),
            form_view,
            image(image_source).alt("Project logo"),
            progress_bar(0.5).label("Half"),
            button("OK"),
        ]));
        let _ = view;

        let macro_view: View = view! {
            <Form>
                <Input value="Ada" />
                <TextArea value="Notes" />
                <Image src="assets/logo.png" alt="Project logo" />
                <ProgressBar value={0.5} label="Half" />
            </Form>
        };
        let _ = macro_view;

        let style = TuiStyle::new()
            .foreground(Color::LightCyan)
            .background(Color::Black)
            .modifier(Modifier::BOLD)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .padding(TuiSpacing::uniform(1));
        let _ = style.to_block();

        let stylesheet = stylesheet! {
            Form => { fg: Color::LightCyan }
            Input => { fg: Color::White }
            TextArea => { fg: Color::Yellow }
            Image => { fg: Color::Green }
            ProgressBar => { fg: Color::Blue }
        };
        assert_eq!(
            stylesheet,
            Stylesheet::new()
                .rule(
                    StyleSelector::view_type(ViewType::Form),
                    TuiStyle::new().foreground(Color::LightCyan),
                )
                .rule(
                    StyleSelector::view_type(ViewType::Input),
                    TuiStyle::new().foreground(Color::White),
                )
                .rule(
                    StyleSelector::view_type(ViewType::TextArea),
                    TuiStyle::new().foreground(Color::Yellow),
                )
                .rule(
                    StyleSelector::view_type(ViewType::Image),
                    TuiStyle::new().foreground(Color::Green),
                )
                .rule(
                    StyleSelector::view_type(ViewType::ProgressBar),
                    TuiStyle::new().foreground(Color::Blue),
                )
        );
    });
}

/// Verifies the prelude exposes resource helpers.
///
/// # Example Under Test
///
/// ```text
/// use leptatui::prelude::*;
/// create_resource(|| (), |_| async { Ok(42) })
/// ```
///
/// # Assertions
///
/// - A resource can be created through prelude exports.
/// - The resource eventually resolves to `ResourceState::Ready(42)`.
#[tokio::test(flavor = "current_thread")]
async fn prelude_exposes_resource_helpers() {
    let owner = Owner::new();

    let resource: Resource<i32, &'static str> =
        owner.with(|| create_resource(|| (), |_| async { Ok(42) }));

    wait_until(|| matches!(resource.get_untracked(), ResourceState::Ready(42))).await;
}
