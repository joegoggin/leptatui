//! Behavior tests for implicit view expression and property tracking.

mod support;

use std::{cell::Cell, rc::Rc};

use leptatui::prelude::*;
use ratatui::{Terminal, backend::TestBackend};
use support::{draw_view, key, rendered_text};

/// Verifies controlled fields and status track ordinary expressions.
///
/// # Example Under Test
///
/// ```text
/// Input value={title.get()}, TextArea value={body.get()}, Text {status}
/// Tab, i, x, y, Esc, Tab, i, z
/// ```
///
/// # Assertions
///
/// - Consecutive input events update caller state without an intervening draw.
/// - External writes update both fields and the status text.
/// - Focus and insert mode survive accepted edits.
#[test]
fn controlled_form_tracks_values_without_a_consumer_closure() -> leptatui::Result<()> {
    let owner = Owner::new();
    owner.with(|| {
        let title = RwSignal::new(String::new());
        let body = RwSignal::new(String::new());
        let status = RwSignal::new(String::from("Editing"));
        let mut view = view! {
            <Div>
                <Text>"My notes"</Text>
                <Form
                    on_submit=move || { status.set(String::from("Submitted")); AppControl::Continue }
                    on_cancel=move || { status.set(String::from("Canceled")); AppControl::Continue }
                >
                    <Text>"Title"</Text>
                    <Input value={title.get()} placeholder="Title"
                        on_input=move |next| { title.set(next); AppControl::Continue } />
                    <Text>"Body"</Text>
                    <TextArea value={body.get()} placeholder="Write a note"
                        on_input=move |next| { body.set(next); AppControl::Continue } />
                </Form>
                <Text>{status}</Text>
            </Div>
        };
        let mut terminal = Terminal::new(TestBackend::new(60, 12))?;
        draw_view(&mut terminal, &view)?;
        for code in [KeyCode::Tab, KeyCode::Char('i'), KeyCode::Char('x'), KeyCode::Char('y')] {
            View::handle_event(&mut view, key(code))?;
        }
        assert_eq!(title.get(), "xy");
        View::handle_event(&mut view, key(KeyCode::Esc))?;
        View::handle_event(&mut view, key(KeyCode::Tab))?;
        View::handle_event(&mut view, key(KeyCode::Char('i')))?;
        View::handle_event(&mut view, key(KeyCode::Char('z')))?;
        assert_eq!(body.get(), "z");
        title.set(String::from("External title"));
        body.set(String::from("External body"));
        status.set(String::from("Synced"));
        draw_view(&mut terminal, &view)?;
        let output = rendered_text(&terminal);
        assert!(output.contains("External title"), "{output}");
        assert!(output.contains("External body"), "{output}");
        assert!(output.contains("Synced"), "{output}");
        Ok(())
    })
}

/// Verifies rejected controlled edits do not alter the displayed value.
///
/// # Example Under Test
///
/// ```text
/// Input value={value.get()} with an on_input handler that rejects changes.
/// ```
///
/// # Assertions
///
/// - Typing invokes the handler but leaves the supplied value intact.
/// - The callback retains its state across externally supplied value changes.
#[test]
fn rejected_edits_and_retained_callbacks() -> leptatui::Result<()> {
    Owner::new().with(|| {
        let value = RwSignal::new(String::from("a"));
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        let mut view = view! {
            <Input value={value.get()} on_input=move |_| {
                calls.set(calls.get() + 1);
                AppControl::Continue
            } />
        };
        for code in [KeyCode::Tab, KeyCode::Char('i'), KeyCode::Char('x')] {
            View::handle_event(&mut view, key(code))?;
        }
        assert_eq!(value.get(), "a");
        assert_eq!(observed.get(), 1);
        value.set(String::from("b"));
        View::handle_event(&mut view, key(KeyCode::Char('y')))?;
        assert_eq!(observed.get(), 2);
        let mut terminal = Terminal::new(TestBackend::new(10, 3))?;
        draw_view(&mut terminal, &view)?;
        assert!(!rendered_text(&terminal).contains('y'));
        Ok(())
    })
}

/// Verifies expressions update text, metadata, and structural collections.
///
/// # Example Under Test
///
/// ```text
/// Text {format!(...)}, reactive class/style, and a conditional Vec of children.
/// ```
///
/// # Assertions
///
/// - Helper-function signal reads invalidate the view.
/// - Changing a collection replaces its visible rows.
/// - Style changes reach the retained rendered text.
#[test]
fn computed_text_styles_and_collections_track_dependencies() -> leptatui::Result<()> {
    Owner::new().with(|| {
        let count = RwSignal::new(1);
        let color = RwSignal::new(Color::Red);
        let view = view! {
            <Div>
                <Text class={format!("count-{}", count.get())}
                    style={TuiStyle::new().foreground(color.get())}>{format!("Count: {}", read_count(count))}</Text>
                { (0..count.get()).map(|index| text(format!("Row {index}"))).collect::<Vec<_>>() }
            </Div>
        };
        let mut terminal = Terminal::new(TestBackend::new(30, 6))?;
        draw_view(&mut terminal, &view)?;
        assert!(rendered_text(&terminal).contains("Count: 1"));
        assert_eq!(terminal.backend().buffer()[(0, 0)].fg, Color::Red);
        count.set(3);
        color.set(Color::Green);
        draw_view(&mut terminal, &view)?;
        let output = rendered_text(&terminal);
        assert!(output.contains("Count: 3"), "{output}");
        assert!(output.contains("Row 2"), "{output}");
        assert_eq!(terminal.backend().buffer()[(0, 0)].fg, Color::Green);
        Ok(())
    })
}

/// Reads a signal indirectly to exercise runtime dependency discovery.
///
/// # Arguments
///
/// * `count` — Counter read by the view's helper expression.
///
/// # Returns
///
/// An integer counter value.
fn read_count(count: RwSignal<i32>) -> i32 {
    count.get()
}

/// Retains local state while rendering a changing prop.
///
/// # Arguments
///
/// * `label` — Parent-owned reactive label.
/// * `setups` — Probe recording construction count.
///
/// # Returns
///
/// A view with local editable state and a changing label.
#[component]
fn StatefulLabel(
    #[prop(into)] label: Prop<String>,
    #[prop(static)] setups: Rc<Cell<usize>>,
) -> impl IntoView {
    setups.set(setups.get() + 1);
    let local = RwSignal::new(String::new());
    view! {
        <Div>
            <Text>{format!("Label: {}", label.get())}</Text>
            <Input value={local.get()} on_input=move |next| { local.set(next); AppControl::Continue } />
            <Text>{format!("Local: {}", local.get())}</Text>
        </Div>
    }
}

/// Verifies changing custom data props retains component setup and state.
///
/// # Example Under Test
///
/// ```text
/// StatefulLabel label={parent_label.get()} with a locally controlled input.
/// ```
///
/// # Assertions
///
/// - The parent label updates after a signal write.
/// - Local edits survive that prop change.
/// - Component setup executes once.
#[test]
fn custom_prop_updates_preserve_component_state() -> leptatui::Result<()> {
    Owner::new().with(|| {
        let label = RwSignal::new(String::from("First"));
        let setups = Rc::new(Cell::new(0));
        let observed = setups.clone();
        let mut view = view! { <Div><StatefulLabel label={label.get()} setups=setups /></Div> };
        let mut terminal = Terminal::new(TestBackend::new(30, 6))?;
        draw_view(&mut terminal, &view)?;
        for code in [KeyCode::Tab, KeyCode::Char('i'), KeyCode::Char('a')] {
            View::handle_event(&mut view, key(code))?;
        }
        label.set(String::from("Second"));
        draw_view(&mut terminal, &view)?;
        let output = rendered_text(&terminal);
        assert!(output.contains("Label: Second"), "{output}");
        assert!(output.contains("Local: a"), "{output}");
        assert_eq!(observed.get(), 1);
        Ok(())
    })
}

include!("fixtures/view_macro/inspect.rs");

/// Verifies data attributes update the retained node across supported leaf types.
///
/// # Example Under Test
///
/// ```text
/// Reactive Input placeholder/id/class/style, Image source/alt,
/// ProgressBar ratio/label, CodeBlock language/line numbers, and Link target.
/// ```
///
/// # Assertions
///
/// - Changed attributes reach the materialized concrete node.
/// - Progress values remain clamped and code presentation changes are applied.
/// - Text and link destinations update together.
#[test]
fn data_attributes_track_without_explicit_boundaries() {
    Owner::new().with(|| {
        let value = RwSignal::new(String::from("First"));
        let color = RwSignal::new(Color::Red);
        let ratio = RwSignal::new(0.5);
        let numbered = RwSignal::new(false);
        let field = view! {
            <Input value={value.get()} placeholder={value.get()} id={value.get()}
                class={value.get()} style={TuiStyle::new().foreground(color.get())} />
        };
        let picture = view! { <Image src={value.get()} alt={value.get()} /> };
        let meter = view! { <ProgressBar value={ratio.get()} label={value.get()} /> };
        let code = view! { <CodeBlock language={value.get()} line_numbers={numbered.get()}>{value.get()}</CodeBlock> };
        let link = view! { <Link href={format!("https://example.com/{}", value.get())}>{value.get()}</Link> };
        inspect::<InputView>(&field, |node| {
            assert_eq!(node.value(), "First");
            assert_eq!(node.editable_state().cursor(), "First".len());
        });
        inspect::<ImageView>(&picture, |node| assert_eq!(node.alt_text(), Some("First")));
        inspect::<ProgressBarView>(&meter, |node| assert_eq!(node.value(), 0.5));
        inspect::<CodeBlockView>(&code, |node| assert_eq!(node.source(), "First"));
        inspect::<LinkView>(&link, |node| assert_eq!(node.content().to_string(), "First"));
        value.set(String::from("rust"));
        color.set(Color::Green);
        ratio.set(2.0);
        numbered.set(true);
        inspect::<InputView>(&field, |node| {
            assert_eq!(node.value(), "rust");
            assert_eq!(node.placeholder_text(), Some("rust"));
            assert_eq!(node.metadata().id(), Some("rust"));
            assert_eq!(node.metadata().classes(), &[String::from("rust")]);
            assert_eq!(node.metadata().inline_style(), Some(TuiStyle::new().foreground(Color::Green)));
        });
        inspect::<ImageView>(&picture, |node| {
            assert_eq!(node.source(), &ImageSource::Path("rust".into()));
            assert_eq!(node.alt_text(), Some("rust"));
        });
        inspect::<ProgressBarView>(&meter, |node| {
            assert_eq!(node.value(), 1.0);
            assert_eq!(node.label_text(), Some("rust"));
        });
        inspect::<CodeBlockView>(&code, |node| {
            assert_eq!(node.source(), "rust");
            assert_eq!(node.language_token(), Some("rust"));
            assert!(node.has_line_numbers());
        });
        inspect::<LinkView>(&link, |node| {
            assert_eq!(node.content().to_string(), "rust");
            assert_eq!(node.target(), &LinkTarget::Url(String::from("https://example.com/rust")));
        });
    });
}

/// Verifies semantic containers retain their layout through reactive updates.
///
/// # Example Under Test
///
/// ```text
/// A reactive table cell, ordered-list start, list-item class, and nested list.
/// ```
///
/// # Assertions
///
/// - Declarative output equals equivalent concrete builder output.
/// - Text, alignment, colors, and list markers update after signal writes.
#[test]
fn semantic_table_and_list_bindings_preserve_layout() {
    Owner::new().with(|| {
        let label = RwSignal::new(String::from("First"));
        let alignment = RwSignal::new(CellAlignment::Right);
        let color = RwSignal::new(Color::Red);
        let start = RwSignal::new(3_usize);
        let actual = view! {
            <Div>
                <Table>
                    <TableHead><TableRow><TableCell>"Name"</TableCell><TableCell>"Status"</TableCell></TableRow></TableHead>
                    <TableBody><TableRow>
                        <TableCell>{label}</TableCell>
                        <TableCell alignment={alignment.get()} style={TuiStyle::new().foreground(color.get())}>{label.get()}</TableCell>
                    </TableRow></TableBody>
                </Table>
                <OrderedList start={start.get()}>
                    <ListItem class={label.get()} style={TuiStyle::new().foreground(color.get())}>
                        <Paragraph>{label.get()}</Paragraph>
                        <UnorderedList style={TuiStyle::new().foreground(color.get())}>
                            <ListItem><Paragraph>"Nested"</Paragraph></ListItem>
                        </UnorderedList>
                    </ListItem>
                </OrderedList>
            </Div>
        };
        for (next_label, next_color, next_alignment, next_start) in [
            ("First", Color::Red, CellAlignment::Right, 3),
            ("Updated", Color::Green, CellAlignment::Center, 12),
        ] {
            label.set(next_label.to_string());
            color.set(next_color);
            alignment.set(next_alignment);
            start.set(next_start);
            let expected = div((
                table((
                    table_head([table_row([table_cell("Name"), table_cell("Status")])]),
                    table_body([table_row([
                        table_cell(next_label),
                        table_cell(next_label).alignment(next_alignment).with_inline_style(TuiStyle::new().foreground(next_color)),
                    ])]),
                )),
                ordered_list([list_item((
                    paragraph(next_label),
                    unordered_list([list_item([paragraph("Nested")])]).with_inline_style(TuiStyle::new().foreground(next_color)),
                )).with_classes(next_label).with_inline_style(TuiStyle::new().foreground(next_color))]).start(next_start),
            ));
            assert_render_eq(&actual, &expected);
        }
    });
}

/// Verifies attribute updates retain all editor state rather than reconstructing it.
///
/// # Example Under Test
///
/// ```text
/// A controlled input is edited, enters visual selection, and changes class.
/// ```
///
/// # Assertions
///
/// - Cursor, selection, undo history, Vim mode, and viewport offsets are retained.
/// - Focus stays on the same control after the binding refresh.
#[test]
fn property_updates_preserve_editor_state() -> leptatui::Result<()> {
    Owner::new().with(|| {
        let value = RwSignal::new(String::new());
        let class = RwSignal::new(String::from("before"));
        let mut field = view! { <Input value={value.get()} class={class.get()}
        on_input=move |next| { value.set(next); AppControl::Continue } /> };
        for code in [
            KeyCode::Tab,
            KeyCode::Char('i'),
            KeyCode::Char('a'),
            KeyCode::Char('b'),
            KeyCode::Esc,
            KeyCode::Char('v'),
        ] {
            View::handle_event(&mut field, key(code))?;
        }
        let mut before = None;
        inspect::<InputView>(&field, |node| {
            assert!(node.metadata().is_focused());
            assert_eq!(node.editable_state().mode(), VimMode::Visual);
            assert!(!node.editable_state().undo_stack().is_empty());
            before = Some(node.editable_state().clone());
        });
        class.set(String::from("after"));
        inspect::<InputView>(&field, |node| {
            assert!(node.metadata().is_focused());
            assert_eq!(node.editable_state(), before.as_ref().unwrap());
            assert_eq!(node.metadata().classes(), &[String::from("after")]);
        });
        Ok(())
    })
}

/// Verifies dependency branches update subscriptions and callbacks stay untracked.
///
/// # Example Under Test
///
/// ```text
/// Input value={if choose_left.get() { left.get() } else { right.get() }}
/// ```
///
/// # Assertions
///
/// - Writes to inactive branches do not reevaluate the binding.
/// - Switching branches replaces the binding's dependencies.
/// - Signal reads during an input callback do not subscribe the binding.
#[test]
fn conditional_dependencies_and_callback_reads_are_isolated() -> leptatui::Result<()> {
    Owner::new().with(|| {
        let left = RwSignal::new(String::from("left"));
        let right = RwSignal::new(String::from("right"));
        let choose_left = RwSignal::new(true);
        let event_only = RwSignal::new(0);
        let evaluations = Rc::new(Cell::new(0));
        let calls = evaluations.clone();
        let mut field = view! {
            <Input value={{
                evaluations.set(evaluations.get() + 1);
                if choose_left.get() { left.get() } else { right.get() }
            }} on_input=move |_| { let _ = event_only.get(); AppControl::Continue } />
        };
        inspect::<InputView>(&field, |node| assert_eq!(node.value(), "left"));
        assert_eq!(calls.get(), 1);
        right.set(String::from("inactive"));
        inspect::<InputView>(&field, |node| assert_eq!(node.value(), "left"));
        assert_eq!(calls.get(), 1);
        choose_left.set(false);
        inspect::<InputView>(&field, |node| assert_eq!(node.value(), "inactive"));
        assert_eq!(calls.get(), 2);
        left.set(String::from("unused"));
        for code in [KeyCode::Tab, KeyCode::Char('i'), KeyCode::Char('x')] {
            View::handle_event(&mut field, key(code))?;
        }
        event_only.set(10);
        inspect::<InputView>(&field, |node| assert_eq!(node.value(), "inactive"));
        assert_eq!(calls.get(), 2);
        Ok(())
    })
}

/// Renders defaulted data props and a retained callback.
///
/// # Arguments
///
/// * `prefix` — Optional prefix defaulting to an empty string.
/// * `label` — Label defaulting to `fallback`.
/// * `action` — Event callback retained independently of the data props.
///
/// # Returns
///
/// A button displaying both current data values.
#[component]
fn DefaultProps(
    #[prop(optional, into)] prefix: Prop<String>,
    #[prop(default = String::from("fallback"), into)] label: Prop<String>,
    #[prop(callback)] action: fn() -> AppControl,
) -> impl IntoView {
    view! { <Button on_press=action>{format!("{}{}", prefix.get(), label.get())}</Button> }
}

/// Verifies defaults, conversions, and callbacks work through reactive props.
///
/// # Example Under Test
///
/// ```text
/// DefaultProps action=|| AppControl::Continue
/// DefaultProps prefix="Hi " label={label.get()} action=|| AppControl::Continue
/// ```
///
/// # Assertions
///
/// - Optional and explicit defaults produce the intended text.
/// - String conversion happens for constant and changing props.
/// - Constant prop clones support tracked `.get()` and borrowed `.with()` reads.
#[test]
fn prop_defaults_conversions_and_callback_coercions() {
    Owner::new().with(|| {
        let defaults = view! { <DefaultProps action=|| AppControl::Continue /> };
        inspect::<ButtonView>(defaults.as_view(), |node| assert_eq!(node.label(), "fallback"));
        let label = RwSignal::new(String::from("First"));
        let changing = view! { <DefaultProps prefix="Hi " label={label.get()} action=|| AppControl::Continue /> };
        inspect::<ButtonView>(changing.as_view(), |node| assert_eq!(node.label(), "Hi First"));
        label.set(String::from("Second"));
        inspect::<ButtonView>(changing.as_view(), |node| assert_eq!(node.label(), "Hi Second"));
        let constant = Prop::new(String::from("constant"));
        assert_eq!(constant.clone().get(), "constant");
        assert_eq!(constant.with(String::len), 8);
    });
}
